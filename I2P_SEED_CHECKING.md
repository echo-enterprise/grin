# I2P Seed Checking for Grin

This document explains how to use the new I2P seed checking functionality that has been added to the Grin project.

## Overview

The I2P seed checking functionality allows you to check the health of Grin seed nodes over the I2P network, providing an additional layer of privacy and censorship resistance compared to regular DNS-based seed checking.

## New Functions

### `check_seed_health_i2p`

This function attempts to connect to a single I2P destination and check its health.

```rust
pub async fn check_seed_health_i2p(
    destination: &str,
    is_testnet: bool,
    samv3_port: Option<u16>,
    timeout_secs: Option<u64>,
) -> Result<(), SeedCheckError>
```

**Parameters:**
- `destination`: The I2P destination (b32.i2p address) to connect to
- `is_testnet`: Whether this is a testnet connection
- `samv3_port`: The SAMv3 port to use for I2P communication (default: 7656)
- `timeout_secs`: Connection timeout in seconds (default: 5)

**Returns:**
- `Result<(), SeedCheckError>`: Success or error with details

### `check_i2p_seeds`

This function checks the health of multiple I2P seed destinations.

```rust
pub async fn check_i2p_seeds(
    destinations: Vec<String>,
    is_testnet: bool,
    samv3_port: Option<u16>,
) -> Vec<SeedCheckResult>
```

**Parameters:**
- `destinations`: Vector of I2P destinations to check
- `is_testnet`: Whether these are testnet destinations
- `samv3_port`: The SAMv3 port to use for I2P communication

**Returns:**
- `Vec<SeedCheckResult>`: Results for each destination

### `check_all_seeds`

This function checks both regular DNS-based seeds and I2P seeds, providing a comprehensive view.

```rust
pub async fn check_all_seeds(
    is_testnet: bool,
    samv3_port: Option<u16>,
    i2p_destinations: Option<Vec<String>>,
) -> SeedCheckResults
```

**Parameters:**
- `is_testnet`: Whether to check testnet or mainnet seeds
- `samv3_port`: The SAMv3 port to use for I2P communication
- `i2p_destinations`: Optional list of I2P destinations to check

**Returns:**
- `SeedCheckResults`: Combined results for both regular and I2P seeds

## Usage Example

Here's how you can use the I2P seed checking functionality:

```rust
use grin_bin::tools::seedcheck::{check_i2p_seeds, check_all_seeds};

#[tokio::main]
async fn main() {
    // Example I2P destinations (replace with actual Grin I2P seed destinations)
    let i2p_destinations = vec![
        "your-mainnet-seed-1.b32.i2p".to_string(),
        "your-mainnet-seed-2.b32.i2p".to_string(),
    ];
    
    // Check only I2P seeds
    let i2p_results = check_i2p_seeds(i2p_destinations.clone(), false, Some(7656)).await;
    
    // Check both regular and I2P seeds
    let all_results = check_all_seeds(false, Some(7656), Some(i2p_destinations)).await;
    
    println!("I2P Results: {:?}", i2p_results);
    println!("All Results: {:?}", all_results);
}
```

## Prerequisites

1. **I2P Router**: You need to have an I2P router running (like the one included in this project)
2. **SAMv3 Port**: The I2P router must be configured to expose the SAMv3 interface (default port: 7656)
3. **I2P Destinations**: You need the b32.i2p addresses of the Grin seed nodes you want to check

## Configuration

### I2P Router Setup

Make sure your I2P router is running and configured with:

```toml
# In your I2P router configuration
[sam]
port = 7656
```

### Finding I2P Seed Destinations

To find I2P seed destinations for Grin, you can:

1. Check the Grin community forums and documentation
2. Look for I2P-specific seed lists
3. Use the I2P address book to resolve .i2p hostnames to b32.i2p addresses

## Error Handling

The functions return `SeedCheckError` which includes:

- `SeedConnectError`: Errors related to seed connection issues
- `StoreError`: Errors related to storage operations
- `I2PError`: Errors related to I2P communication

## Integration with Existing Code

The new I2P functions are designed to work alongside the existing seed checking functionality. You can:

1. Use them independently for I2P-only checking
2. Combine them with regular DNS-based checking for comprehensive coverage
3. Integrate them into your existing seed health monitoring systems

## Security Considerations

- I2P provides additional privacy by routing traffic through the I2P network
- The SAMv3 interface should be properly secured and not exposed to the public internet
- Consider using different I2P destinations for testnet and mainnet to avoid confusion

## Troubleshooting

### Common Issues

1. **Connection Timeout**: Check that your I2P router is running and the SAMv3 port is accessible
2. **Destination Not Found**: Verify that the b32.i2p addresses are correct and the destinations are online
3. **Permission Denied**: Ensure your application has permission to connect to the SAMv3 port

### Debugging

Enable debug logging to see detailed connection information:

```rust
// Set up logging with debug level
env_logger::init();
```

## Future Enhancements

Potential improvements to consider:

1. Automatic discovery of I2P seed destinations
2. Integration with the I2P address book for hostname resolution
3. Support for I2P-specific connection options and configurations
4. Metrics and monitoring for I2P connection health 