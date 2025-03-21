mod interfaces;
mod stores;

use anyhow::Result;
use interfaces::http::HttpInterface;
use stores::hashmap::HashmapStore;

#[tokio::main]
async fn main() -> Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let hashmap_store = HashmapStore::new()?;

    let http_interface = HttpInterface {};

    tokio::join!(http_interface.start_app(hashmap_store))
        .0
        .await??;

    Ok(())
}
