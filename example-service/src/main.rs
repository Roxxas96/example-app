mod clients;
mod interfaces;
mod stores;

use std::{net::AddrParseError, sync::Arc};

use anyhow::Result;
use clients::grpc::{GrpcClient, GrpcClientError};
use config::{Config, ConfigError};
use interfaces::{
    grpc::{word::word_service_server::WordServiceServer, GrpcInterface, GrpcInterfaceError},
    http::{HttpInterface, HttpInterfaceError},
};
use serde::{Deserialize, Serialize};
use stores::hashmap::{HashmapStore, HashmapStoreError};
use thiserror::Error;
use tokio::{
    sync::Mutex,
    task::{JoinError, JoinHandle},
};
use tonic::transport::Server;
use tracing::{debug, info};

#[derive(Error, Debug)]
enum ExampleAppError {
    #[error("Failed to build configuration: {0}")]
    ConfigBuildError(ConfigError),
    #[error("Failed to build hashmap store: {0}")]
    HashmapStoreError(HashmapStoreError),
    #[error("Failed to create gRPC client: {0}")]
    GrpcClientError(GrpcClientError),
    #[error("Failed to parse url configuration: {0}")]
    UrlParseError(AddrParseError),
    #[error("Failed to start http server: {0}")]
    HttpServerStartError(HttpInterfaceError),
    #[error("Failed to start gRPC server: {0}")]
    GrpcServerStartError(GrpcInterfaceError),
    #[error("Failed to start join handle: {0}")]
    JoinHandleError(JoinError),
}

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
        .build()
        .map_err(ExampleAppError::ConfigBuildError)?
        .try_deserialize()
        .map_err(ExampleAppError::ConfigBuildError)?;

    let hashmap_store = Arc::new(Mutex::new(
        HashmapStore::new().map_err(ExampleAppError::HashmapStoreError)?,
    ));

    let http_interface = HttpInterface::new(hashmap_store.clone());
    let http_server = http_interface.start_app(config.http_port).await;

    let grpc_clients = Arc::new(Mutex::new(Vec::new()));
    let grpc_url = format!("0.0.0.0:{0}", config.grpc_port)
        .parse()
        .map_err(ExampleAppError::UrlParseError)?;

    // Start a task for connecting the gRPC clients asynchronously
    let client_connect_task: JoinHandle<Result<(), GrpcClientError>> = tokio::spawn({
        let grpc_clients = grpc_clients.clone();
        async move {
            let mut clients = Vec::new();
            for service_url in config.connected_services {
                clients.push(GrpcClient::new(service_url).await?)
            }
            *grpc_clients.lock().await = clients;
            Ok(())
        }
    });

    // Now, we can start the gRPC server, even before the clients are connected
    let grpc_interface = GrpcInterface::new(hashmap_store.clone(), grpc_clients.clone());
    let grpc_server = tokio::spawn(async move {
        debug!("Starting gRPC interface on port {0}...", config.grpc_port);
        Server::builder()
            .add_service(WordServiceServer::new(grpc_interface))
            .serve(grpc_url)
            .await
            .map_err(GrpcInterfaceError::GrpcServerError)
    });

    let join_handle = tokio::join!(http_server, grpc_server, client_connect_task);

    join_handle
        .0
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::HttpServerStartError)?;
    join_handle
        .1
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::GrpcServerStartError)?;
    join_handle
        .2
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::GrpcClientError)?;

    Ok(())
}
