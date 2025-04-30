mod clients;
mod core;
mod interfaces;
mod stores;

use std::{net::AddrParseError, sync::Arc};

use crate::core::Core;
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

    info!("Building config...");
    let config: ExampleAppConfig = Config::builder()
        .add_source(
            config::Environment::default()
                .try_parsing(true)
                .list_separator(","),
        )
        .set_default("http_port", 3001)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("grpc_port", 50051)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("connected_services", Vec::<String>::new())
        .map_err(ExampleAppError::ConfigError)?
        .build()
        .map_err(ExampleAppError::ConfigError)?
        .try_deserialize()
        .map_err(ExampleAppError::ConfigError)?;

    info!("Building hashmap store...");
    let hashmap_store = HashmapStore::new().map_err(ExampleAppError::HashmapStoreError)?;

    let core = Arc::new(Mutex::new(Core::new(
        hashmap_store,
        config.connected_services,
    )));

    let http_interface = HttpInterface::new(core.clone());
    let http_server = http_interface.start_app(config.http_port).await;

    let grpc_interface = GrpcInterface::new(core.clone());
    let grpc_url = format!("0.0.0.0:{0}", config.grpc_port)
        .parse()
        .map_err(|e| ExampleAppError::UrlParseError {
            source: e,
            port: config.grpc_port,
        })?;
    let grpc_server = tokio::spawn(async move {
        info!("Starting gRPC interface on address {0}...", grpc_url);
        Server::builder()
            .add_service(WordServiceServer::new(grpc_interface))
            .serve(grpc_url)
            .await
            .map_err(|e| GrpcInterfaceError::GrpcServerError {
                source: e,
                address: grpc_url,
            })
    });

    let join_handle = tokio::join!(http_server, grpc_server);

    join_handle
        .0
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::HttpServerError)?;
    join_handle
        .1
        .map_err(ExampleAppError::JoinHandleError)?
        .map_err(ExampleAppError::GrpcServerError)?;

    Ok(())
}
