// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

#![allow(clippy::crate_in_macro_def)]
#![allow(clippy::too_many_arguments)]

use i2p_router::{router_event_loop, setup_router, ui::web::RouterUi, RouterContext};
use std::sync::OnceLock;
use tokio::sync::mpsc::{channel, Receiver};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn start_i2p_router() -> anyhow::Result<()> {
	let runtime =
		RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create runtime"));
	let (shutdown_tx, shutdown_rx) = channel(1);
	let RouterContext {
		router,
		port_mapper,
		events,
		router_ui_config,
	} = runtime.block_on(setup_router())?;

	// Spawn the UI task (infinite loop)
	runtime.spawn(async move {
		RouterUi::new(events, Some(7657), 5, shutdown_tx)
			.run()
			.await;
	});

	// Spawn the router event loop task (infinite loop)
	runtime.spawn(async move {
		router_event_loop(router, port_mapper, shutdown_rx).await;
	});

	// Return immediately, letting both tasks run in the background
	// The runtime will stay alive because it's stored in the static variable
	Ok(())
}
