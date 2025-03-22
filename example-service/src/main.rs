mod interfaces;
mod stores;

use anyhow::Result;
use interfaces::http::HttpInterface;
use stores::hashmap::HashmapStore;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting example service...");

    let hashmap_store = HashmapStore::new()?;

    let http_interface = HttpInterface::new(hashmap_store);

    tokio::join!(http_interface.start_app()).0.await??;

    Ok(())
}
