mod clients;
mod interfaces;
mod stores;

use std::sync::Arc;

use anyhow::Result;
use clients::grpc::GrpcClient;
use config::Config;
use interfaces::{
    grpc::{word::word_service_server::WordServiceServer, GrpcInterface},
    http::HttpInterface,
};
use serde::{Deserialize, Serialize};
use stores::hashmap::HashmapStore;
use tokio::sync::Mutex;
use tonic::transport::Server;
use tracing::{debug, info};

#[derive(Serialize, Deserialize, Debug)]
struct ExampleAppConfig {
    http_port: u16,
    grpc_port: u16,
    connected_services: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting example service...");

    let config: ExampleAppConfig = Config::builder()
        .add_source(
            config::Environment::default()
                .try_parsing(true)
                .list_separator(","),
        )
        .set_default("http_port", 3000)?
        .set_default("grpc_port", 50051)?
        .set_default("connected_services", Vec::<String>::new())?
        .build()?
        .try_deserialize()?;

    let hashmap_store = Arc::new(Mutex::new(HashmapStore::new()?));

    let http_interface = HttpInterface::new(hashmap_store.clone());
    let http_server = http_interface.start_app(config.http_port);

    let mut grpc_clients = vec![];
    for service_url in config.connected_services {
        grpc_clients.push(Arc::new(Mutex::new(GrpcClient::new(service_url).await)));
    }

    let grpc_interface = GrpcInterface::new(hashmap_store.clone(), grpc_clients);
    let grpc_server = tokio::spawn(async move {
        debug!("Starting gRPC interface on port {0}...", config.grpc_port);
        Server::builder()
            .add_service(WordServiceServer::new(grpc_interface))
            .serve(format!("0.0.0.0:{0}", config.grpc_port).parse().unwrap())
            .await
    });

    let join_handle = tokio::join!(http_server, grpc_server);

    join_handle.0.await??;
    join_handle.1??;

    Ok(())
}
