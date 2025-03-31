mod clients;
mod interfaces;
mod stores;

use std::{net::AddrParseError, sync::Arc};

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
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
enum ExampleAppError {
    #[error("Config error")]
    ConfigError(#[source] ConfigError),
    #[error("Hashmap store error")]
    HashmapStoreError(#[source] HashmapStoreError),
    #[error("gRPC client error")]
    GrpcClientError(#[source] GrpcClientError),
    #[error("Failed to parse url for port {port:?}")]
    UrlParseError {
        #[source]
        source: AddrParseError,
        port: u16,
    },
    #[error("HTTP server error")]
    HttpServerError(#[source] HttpInterfaceError),
    #[error("gRPC server error")]
    GrpcServerError(#[source] GrpcInterfaceError),
    #[error("Join error")]
    JoinHandleError(#[source] JoinError),
}

#[derive(Serialize, Deserialize, Debug)]
struct ExampleAppConfig {
    http_port: u16,
    grpc_port: u16,
    connected_services: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting example service...");

    let config: ExampleAppConfig = Config::builder()
        .add_source(
            config::Environment::default()
                .try_parsing(true)
                .list_separator(","),
        )
        .set_default("http_port", 3000)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("grpc_port", 50051)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("connected_services", Vec::<String>::new())
        .map_err(ExampleAppError::ConfigError)?
        .build()
        .map_err(ExampleAppError::ConfigError)?
        .try_deserialize()
        .map_err(ExampleAppError::ConfigError)?;

    let hashmap_store = Arc::new(Mutex::new(
        HashmapStore::new().map_err(ExampleAppError::HashmapStoreError)?,
    ));

    let http_interface = HttpInterface::new(hashmap_store.clone());
    let http_server = http_interface.start_app(config.http_port).await;

    let grpc_clients = Arc::new(Mutex::new(Vec::new()));
    let grpc_url = format!("0.0.0.0:{0}", config.grpc_port)
        .parse()
        .map_err(|e| ExampleAppError::UrlParseError {
            source: e,
            port: config.grpc_port,
        })?;

    let client_connect_task: JoinHandle<Result<(), GrpcClientError>> = tokio::spawn({
        let grpc_clients = grpc_clients.clone();
        async move {
            let mut clients = Vec::new();
            for service_url in config.connected_services {
                clients.push(GrpcClient::new(service_url).await.map_err(|e| {
                    warn!("gRPC client connect failed: {:?}", e);
                    e
                })?)
            }
            *grpc_clients.lock().await = clients;
            Ok(())
        }
    });

    let grpc_interface = GrpcInterface::new(hashmap_store.clone(), grpc_clients.clone());
    let grpc_server = tokio::spawn(async move {
        debug!("Starting gRPC interface on port {0}...", config.grpc_port);
        Server::builder()
            .add_service(WordServiceServer::new(grpc_interface))
            .serve(grpc_url)
            .await
            .map_err(|e| GrpcInterfaceError::GrpcServerError {
                source: e,
                address: grpc_url,
            })
    });

    let join_handle = tokio::join!(http_server, grpc_server, client_connect_task);

    join_handle
        .0
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::HttpServerError)?;
    join_handle
        .1
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::GrpcServerError)?;
    join_handle
        .2
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::GrpcClientError)?;

    Ok(())
}
