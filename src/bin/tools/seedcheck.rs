// Copyright 2024 The Grin Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Relatively self-contained seed health checker
use std::sync::Arc;

use grin_core::core::hash::Hashed;
use grin_core::pow::Difficulty;
use grin_core::{genesis, global};
use grin_p2p as p2p;
use grin_servers::{resolve_dns_to_addrs, MAINNET_DNS_SEEDS, TESTNET_DNS_SEEDS};
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use thiserror::Error;

// Add yosemite dependency for I2P communication
use tokio;
use yosemite::{style, Session, SessionOptions, StreamOptions};

#[derive(Error, Debug)]
pub enum SeedCheckError {
	#[error("Seed Connect Error {0}")]
	SeedConnectError(String),
	#[error("Grin Store Error {0}")]
	StoreError(String),
	#[error("I2P Error {0}")]
	I2PError(String),
}

impl From<p2p::Error> for SeedCheckError {
	fn from(e: p2p::Error) -> Self {
		SeedCheckError::SeedConnectError(format!("{:?}", e))
	}
}

impl From<grin_store::lmdb::Error> for SeedCheckError {
	fn from(e: grin_store::lmdb::Error) -> Self {
		SeedCheckError::StoreError(format!("{:?}", e))
	}
}

impl From<yosemite::Error> for SeedCheckError {
	fn from(e: yosemite::Error) -> Self {
		SeedCheckError::I2PError(format!("{:?}", e))
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SeedCheckResults {
	pub mainnet: Vec<SeedCheckResult>,
	pub testnet: Vec<SeedCheckResult>,
}

impl Default for SeedCheckResults {
	fn default() -> Self {
		Self {
			mainnet: vec![],
			testnet: vec![],
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeedCheckResult {
	pub url: String,
	pub dns_resolutions_found: bool,
	pub success: bool,
	pub successful_attempts: Vec<SeedCheckConnectAttempt>,
	pub unsuccessful_attempts: Vec<SeedCheckConnectAttempt>,
}

impl Default for SeedCheckResult {
	fn default() -> Self {
		Self {
			url: "".into(),
			dns_resolutions_found: false,
			success: false,
			successful_attempts: vec![],
			unsuccessful_attempts: vec![],
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeedCheckConnectAttempt {
	pub ip_addr: String,
	pub handshake_success: bool,
	pub user_agent: Option<String>,
	pub capabilities: Option<String>,
}

pub fn check_seeds(is_testnet: bool) -> Vec<SeedCheckResult> {
	let mut result = vec![];
	let (default_seeds, port) = match is_testnet {
		true => (TESTNET_DNS_SEEDS, "13414"),
		false => (MAINNET_DNS_SEEDS, "3414"),
	};

	if is_testnet {
		global::set_local_chain_type(global::ChainTypes::Testnet);
	}

	let config = p2p::types::P2PConfig::default();
	let adapter = Arc::new(p2p::DummyAdapter {});
	let peers = Arc::new(p2p::Peers::new(
		p2p::store::PeerStore::new(".__grintmp__/peer_store_root").unwrap(),
		adapter,
		config.clone(),
	));

	for s in default_seeds.iter() {
		warn!("Checking seed health for {}", s);
		let mut seed_result = SeedCheckResult::default();
		seed_result.url = s.to_string();
		let resolved_dns_entries = resolve_dns_to_addrs(&vec![format!("{}:{}", s, port)]);
		if resolved_dns_entries.is_empty() {
			warn!("FAIL - No dns entries found for {}", s);
			result.push(seed_result);
			continue;
		}
		seed_result.dns_resolutions_found = true;
		// Check backwards, last contains the latest (at least on my machine!)
		for r in resolved_dns_entries.iter().rev() {
			let res = check_seed_health(*r, is_testnet, &peers);
			if let Ok(p) = res {
				warn!(
					"SUCCESS - Performed Handshake with seed for {} at {}. {} - {:?}",
					s, r, p.info.user_agent, p.info.capabilities
				);
				//warn!("{:?}", p);
				seed_result.success = true;
				seed_result
					.successful_attempts
					.push(SeedCheckConnectAttempt {
						ip_addr: r.to_string(),
						handshake_success: true,
						user_agent: Some(p.info.user_agent),
						capabilities: Some(format!("{:?}", p.info.capabilities)),
					});
			} else {
				seed_result
					.unsuccessful_attempts
					.push(SeedCheckConnectAttempt {
						ip_addr: r.to_string(),
						handshake_success: false,
						user_agent: None,
						capabilities: None,
					});
			}
		}

		if !seed_result.success {
			warn!(
				"FAIL - Unable to handshake at any known DNS resolutions for {}",
				s
			);
		}

		result.push(seed_result);
	}

	// Clean up temporary files
	fs::remove_dir_all(".__grintmp__").expect("Unable to delete temporary files");

	result
}

fn check_seed_health(
	addr: p2p::PeerAddr,
	is_testnet: bool,
	peers: &Arc<p2p::Peers>,
) -> Result<p2p::Peer, SeedCheckError> {
	let config = p2p::types::P2PConfig::default();
	let capabilities = p2p::types::Capabilities::default();
	let genesis_hash = match is_testnet {
		true => genesis::genesis_test().hash(),
		false => genesis::genesis_main().hash(),
	};

	let handshake = p2p::handshake::Handshake::new(genesis_hash, config.clone());

	match TcpStream::connect_timeout(&addr.0, Duration::from_secs(5)) {
		Ok(stream) => {
			let addr = SocketAddr::new(config.host, config.port);
			let total_diff = Difficulty::from_num(1);

			let peer = p2p::Peer::connect(
				stream,
				capabilities,
				total_diff,
				p2p::PeerAddr(addr),
				&handshake,
				peers.clone(),
			)?;
			Ok(peer)
		}
		Err(e) => {
			trace!(
				"connect_peer: on {}:{}. Could not connect to {}: {:?}",
				config.host,
				config.port,
				addr,
				e
			);
			Err(p2p::Error::Connection(e).into())
		}
	}
}

/// Check seed health over I2P network
///
/// This function attempts to connect to a seed node over I2P using the yosemite crate.
/// It creates an I2P session and attempts to establish a connection to the given destination.
///
/// # Arguments
/// * `destination` - The I2P destination (b32.i2p address) to connect to
/// * `is_testnet` - Whether this is a testnet connection
/// * `samv3_port` - The SAMv3 port to use for I2P communication (default: 7656)
/// * `timeout_secs` - Connection timeout in seconds (default: 5)
///
/// # Returns
/// * `Result<(), SeedCheckError>` - Success or error with details
pub async fn check_seed_health_i2p(
	destination: &str,
	is_testnet: bool,
	samv3_port: Option<u16>,
	timeout_secs: Option<u64>,
) -> Result<(), SeedCheckError> {
	let samv3_port = samv3_port.unwrap_or(7656);
	let timeout_secs = timeout_secs.unwrap_or(5);

	// Create I2P session
	let mut session = Session::<style::Stream>::new(SessionOptions {
		publish: false,
		samv3_tcp_port: samv3_port,
		nickname: format!(
			"seedcheck-{}",
			if is_testnet { "testnet" } else { "mainnet" }
		),
		num_inbound: 1,
		num_outbound: 1,
		..Default::default()
	})
	.await?;

	// Attempt to connect to the destination
	let connect_future = session.connect_detached_with_options(
		destination,
		StreamOptions {
			dst_port: 0, // Use default port
			..Default::default()
		},
	);

	// Set up timeout
	let timeout_duration = Duration::from_secs(timeout_secs);
	let _timeout_future = tokio::time::sleep(timeout_duration);

	// Race between connection and timeout
	match tokio::time::timeout(timeout_duration, connect_future).await {
		Ok(Ok(_stream)) => {
			trace!(
				"SUCCESS - Connected to I2P destination {} ({}net)",
				destination,
				if is_testnet { "test" } else { "main" }
			);
			Ok(())
		}
		Ok(Err(e)) => {
			trace!(
				"FAIL - Failed to connect to I2P destination {}: {:?}",
				destination,
				e
			);
			Err(e.into())
		}
		Err(_) => {
			trace!(
				"FAIL - Timeout connecting to I2P destination {} after {} seconds",
				destination,
				timeout_secs
			);
			Err(SeedCheckError::I2PError(format!(
				"Connection timeout after {} seconds",
				timeout_secs
			)))
		}
	}
}

/// Check multiple I2P seeds for health
///
/// This function checks the health of multiple I2P seed destinations.
///
/// # Arguments
/// * `destinations` - Vector of I2P destinations to check
/// * `is_testnet` - Whether these are testnet destinations
/// * `samv3_port` - The SAMv3 port to use for I2P communication
///
/// # Returns
/// * `Vec<SeedCheckResult>` - Results for each destination
pub async fn check_i2p_seeds(
	destinations: Vec<String>,
	is_testnet: bool,
	samv3_port: Option<u16>,
) -> Vec<SeedCheckResult> {
	let mut result = vec![];

	for destination in destinations.iter() {
		warn!("Checking I2P seed health for {}", destination);
		let mut seed_result = SeedCheckResult::default();
		seed_result.url = destination.to_string();

		// For I2P, we don't do DNS resolution, so we assume it's available
		seed_result.dns_resolutions_found = true;

		// Attempt to connect to the I2P destination
		match check_seed_health_i2p(destination, is_testnet, samv3_port, Some(5)).await {
			Ok(()) => {
				warn!(
					"SUCCESS - Connected to I2P seed {} ({})",
					destination,
					if is_testnet { "testnet" } else { "mainnet" }
				);
				seed_result.success = true;
				seed_result
					.successful_attempts
					.push(SeedCheckConnectAttempt {
						ip_addr: destination.clone(),
						handshake_success: true,
						user_agent: Some("I2P Connection".to_string()),
						capabilities: Some("I2P Protocol".to_string()),
					});
			}
			Err(e) => {
				warn!(
					"FAIL - Unable to connect to I2P seed {}: {:?}",
					destination, e
				);
				seed_result
					.unsuccessful_attempts
					.push(SeedCheckConnectAttempt {
						ip_addr: destination.clone(),
						handshake_success: false,
						user_agent: None,
						capabilities: None,
					});
			}
		}

		result.push(seed_result);
	}

	result
}

/// Example function demonstrating how to use I2P seed checking
///
/// This function shows how to check the health of I2P seed destinations.
/// You would typically call this from your main application.
///
/// # Arguments
/// * `is_testnet` - Whether to check testnet or mainnet seeds
/// * `samv3_port` - The SAMv3 port to use for I2P communication
///
/// # Returns
/// * `Vec<SeedCheckResult>` - Results for each I2P destination
pub async fn check_i2p_seeds_example(
	is_testnet: bool,
	samv3_port: Option<u16>,
) -> Vec<SeedCheckResult> {
	// Example I2P destinations - replace with actual Grin I2P seed destinations
	let i2p_destinations = if is_testnet {
		vec![
			// Add your testnet I2P seed destinations here
			// Example: "your-testnet-seed-1.b32.i2p".to_string(),
			// Example: "your-testnet-seed-2.b32.i2p".to_string(),
		]
	} else {
		vec![
			// Add your mainnet I2P seed destinations here
			// Example: "your-mainnet-seed-1.b32.i2p".to_string(),
			// Example: "your-mainnet-seed-2.b32.i2p".to_string(),
		]
	};

	if i2p_destinations.is_empty() {
		warn!(
			"No I2P destinations configured for {}net",
			if is_testnet { "test" } else { "main" }
		);
		return vec![];
	}

	check_i2p_seeds(i2p_destinations, is_testnet, samv3_port).await
}

/// Combined seed checking function that checks both regular and I2P seeds
///
/// This function checks both regular DNS-based seeds and I2P seeds,
/// providing a comprehensive view of seed health across different networks.
///
/// # Arguments
/// * `is_testnet` - Whether to check testnet or mainnet seeds
/// * `samv3_port` - The SAMv3 port to use for I2P communication
/// * `i2p_destinations` - Optional list of I2P destinations to check
///
/// # Returns
/// * `SeedCheckResults` - Combined results for both regular and I2P seeds
pub async fn check_all_seeds(
	is_testnet: bool,
	samv3_port: Option<u16>,
	i2p_destinations: Option<Vec<String>>,
) -> SeedCheckResults {
	let mut results = SeedCheckResults::default();

	// Check regular DNS-based seeds
	let regular_seeds = check_seeds(is_testnet);
	if is_testnet {
		results.testnet.extend(regular_seeds);
	} else {
		results.mainnet.extend(regular_seeds);
	}

	// Check I2P seeds if destinations are provided
	if let Some(destinations) = i2p_destinations {
		let i2p_seeds = check_i2p_seeds(destinations, is_testnet, samv3_port).await;
		if is_testnet {
			results.testnet.extend(i2p_seeds);
		} else {
			results.mainnet.extend(i2p_seeds);
		}
	}

	results
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_check_seed_health_i2p_invalid_destination() {
		// Test with an invalid destination - should fail gracefully
		let result = check_seed_health_i2p("invalid-destination", false, Some(7656), Some(1)).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_check_i2p_seeds_empty_list() {
		// Test with empty destinations list
		let destinations = vec![];
		let results = check_i2p_seeds(destinations, false, Some(7656)).await;
		assert_eq!(results.len(), 0);
	}

	#[tokio::test]
	async fn test_check_all_seeds_no_i2p() {
		// Set global chain type for test
		global::set_local_chain_type(global::ChainTypes::Mainnet);

		// Test check_all_seeds without I2P destinations
		let results = check_all_seeds(false, Some(7656), None).await;
		// Should only contain regular seeds
		assert_eq!(results.mainnet.len(), 0); // No regular seeds in test environment
		assert_eq!(results.testnet.len(), 0);
	}

	#[test]
	fn test_seed_check_error_i2p() {
		// Test I2P error conversion
		let error = SeedCheckError::I2PError("Test error".to_string());
		assert_eq!(error.to_string(), "I2P Error Test error");
	}
}
