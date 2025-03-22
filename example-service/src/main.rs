mod interfaces;
mod stores;

use std::sync::Arc;

use anyhow::Result;
use interfaces::http::HttpInterface;
use stores::hashmap::HashmapStore;
use tokio::sync::Mutex;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting example service...");

    let hashmap_store = Arc::new(Mutex::new(HashmapStore::new()?));

    let http_interface = HttpInterface::new(hashmap_store);

    tokio::join!(http_interface.start_app()).0.await??;

    Ok(())
}
