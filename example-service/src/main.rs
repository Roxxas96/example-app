mod clients;
mod core;
mod interfaces;
mod stores;

use crate::clients::grpc::{GrpcClient, GrpcClientError};
use crate::core::Core;
use config::{Config, ConfigError};
use interfaces::{
    grpc::{word::word_service_server::WordServiceServer, GrpcInterface, GrpcInterfaceError},
    http::{HttpInterface, HttpInterfaceError},
};
use metrics_exporter_prometheus::PrometheusBuilder;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{net::AddrParseError, sync::Arc};
use stores::hashmap::{HashmapStore, HashmapStoreError};
use thiserror::Error;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
enum ExampleAppError {
    #[error("Config error")]
    ConfigError(#[source] ConfigError),
    #[error("Hashmap store error")]
    HashmapStoreError(#[source] HashmapStoreError),
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
    #[error("gRPC client error")]
    GrpcClientError(#[source] GrpcClientError),
    #[error("Error with Prometheus interface")]
    PrometheusError(#[source] metrics_exporter_prometheus::BuildError),
}

#[derive(Serialize, Deserialize, Debug)]
struct ExampleAppConfig {
    http_port: u16,
    grpc_port: u16,
    metrics_port: u16,
    connected_services: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), ExampleAppError> {
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
        .set_default("metrics_port", 9001)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("connected_services", Vec::<String>::new())
        .map_err(ExampleAppError::ConfigError)?
        .build()
        .map_err(ExampleAppError::ConfigError)?
        .try_deserialize()
        .map_err(ExampleAppError::ConfigError)?;

    let metrics_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), config.metrics_port);
    info!(
        "Starting metrics exporter on address {:?}...",
        metrics_address
    );
    PrometheusBuilder::new()
        .with_http_listener(metrics_address)
        .install()
        .map_err(ExampleAppError::PrometheusError)?;

    info!("Building hashmap store...");
    let hashmap_store = HashmapStore::new()
        .await
        .map_err(ExampleAppError::HashmapStoreError)?;

    info!("Building gRPC clients...");
    let grpc_clients = Arc::new(RwLock::new(Vec::new()));
    let grpc_clients_clone = grpc_clients.clone();
    let grpc_clients_task = async move {
        for service_url in config.connected_services {
            grpc_clients_clone.write().await.push(
                GrpcClient::new(service_url.clone())
                    .await
                    .map_err(ExampleAppError::GrpcClientError)?,
            );
            debug!("Connected gRPC client to {:?}", service_url);
        }
        Ok(())
    };

    let core = Core::new(hashmap_store, grpc_clients);

    let http_interface = HttpInterface::new(core.clone());
    let http_server_task = async move {
        http_interface
            .start_app(config.http_port)
            .await
            .map_err(ExampleAppError::HttpServerError)?;
        Ok(())
    };

    let grpc_interface = GrpcInterface::new(core);
    let grpc_url = format!("0.0.0.0:{0}", config.grpc_port)
        .parse()
        .map_err(|e| ExampleAppError::UrlParseError {
            source: e,
            port: config.grpc_port,
        })?;
    let grpc_server_task = async move {
        info!("Starting gRPC interface on address {0}...", grpc_url);
        Server::builder()
            .add_service(WordServiceServer::new(grpc_interface))
            .serve(grpc_url)
            .await
            .map_err(|e| GrpcInterfaceError::GrpcServerError {
                source: e,
                address: grpc_url,
            })
            .map_err(ExampleAppError::GrpcServerError)?;
        Ok(())
    };

    tokio::try_join!(http_server_task, grpc_server_task, grpc_clients_task)?;

    Ok(())
}
