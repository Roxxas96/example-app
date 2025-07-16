mod clients;
mod core;
mod interfaces;
mod stores;

use crate::clients::grpc::{GrpcClient, GrpcClientError};
use crate::clients::Client;
use crate::core::Core;
use crate::stores::Store;
use config::{Config, ConfigError};
use interfaces::{
    grpc::{word::word_service_server::WordServiceServer, GrpcInterface, GrpcInterfaceError},
    http::{HttpInterface, HttpInterfaceError},
};
use metrics_exporter_prometheus::PrometheusBuilder;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{net::AddrParseError, sync::Arc};
use stores::hashmap::{HashmapStore, HashmapStoreError};
use thiserror::Error;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;

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
    #[error("Error when building OpenTelemetry exporter")]
    OtelExporterBuildError(#[source] opentelemetry_otlp::ExporterBuildError),
    #[error("Error when init tracing registry")]
    TracingRegistryInitError(#[source] tracing_subscriber::util::TryInitError),
}

#[derive(Serialize, Deserialize, Debug)]
struct ExampleAppConfig {
    http_port: u16,
    grpc_port: u16,
    metrics_port: u16,
    connected_services: String,
    tracing_endpoint: String,
    service_name: String,
}

fn init_config() -> Result<ExampleAppConfig, ExampleAppError> {
    Ok(Config::builder()
        .add_source(
            config::Environment::default()
                .prefix("EXAMPLE_SERVICE")
                .separator("__"),
        )
        .set_default("http_port", 3001)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("grpc_port", 50051)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("metrics_port", 9001)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("connected_services", "")
        .map_err(ExampleAppError::ConfigError)?
        .set_default("tracing_endpoint", "http://localhost:4317")
        .map_err(ExampleAppError::ConfigError)?
        .set_default("service_name", "example-service-1")
        .map_err(ExampleAppError::ConfigError)?
        .build()
        .map_err(ExampleAppError::ConfigError)?
        .try_deserialize()
        .map_err(ExampleAppError::ConfigError)?)
}

fn init_tracing(config: &ExampleAppConfig) -> Result<(), ExampleAppError> {
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(config.tracing_endpoint.clone())
                .build()
                .map_err(ExampleAppError::OtelExporterBuildError)?,
        )
        .with_resource(
            Resource::builder()
                .with_service_name(config.service_name.clone())
                .build(),
        )
        .build();
    let tracer = provider.tracer("readme_example");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(tracing_subscriber::fmt::Layer::default())
        .with(telemetry)
        .try_init()
        .map_err(ExampleAppError::TracingRegistryInitError)?;
    Ok(())
}

fn init_metrics(config: &ExampleAppConfig) -> Result<(), ExampleAppError> {
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
    Ok(())
}

async fn init_store() -> Result<impl Store, ExampleAppError> {
    info!("Building hashmap store...");
    Ok(HashmapStore::new()
        .await
        .map_err(ExampleAppError::HashmapStoreError)?)
}

fn init_core(
    store: impl Store,
    config: &ExampleAppConfig,
) -> Result<
    (
        Core<impl Store, impl Client>,
        impl Future<Output = Result<(), ExampleAppError>>,
    ),
    ExampleAppError,
> {
    info!("Building gRPC clients...");
    let grpc_clients = Arc::new(RwLock::new(Vec::new()));
    let grpc_clients_task = {
        let grpc_clients_clone = grpc_clients.clone();
        let urls = config.connected_services.clone();
        async move {
            for service_url in urls.split(',') {
                let client = GrpcClient::new(service_url.to_string())
                    .await
                    .map_err(ExampleAppError::GrpcClientError)?;
                grpc_clients_clone.write().await.push(client);
                debug!("Connected gRPC client to {:?}", service_url);
            }
            Result::<(), ExampleAppError>::Ok(())
        }
    };

    Ok((Core::new(store, grpc_clients), grpc_clients_task))
}

fn init_http_interface(
    core: Core<impl Store, impl Client>,
    config: &ExampleAppConfig,
) -> impl Future<Output = Result<(), ExampleAppError>> {
    let http_interface = HttpInterface::new(core.clone());
    let http_port = config.http_port.clone();
    async move {
        http_interface
            .start_app(http_port)
            .await
            .map_err(ExampleAppError::HttpServerError)?;
        Result::<(), ExampleAppError>::Ok(())
    }
}

fn init_grpc_interface(
    core: Core<impl Store, impl Client>,
    config: &ExampleAppConfig,
) -> Result<impl Future<Output = Result<(), ExampleAppError>>, ExampleAppError> {
    let grpc_interface = GrpcInterface::new(core);
    let grpc_url = format!("0.0.0.0:{0}", config.grpc_port)
        .parse()
        .map_err(|e| ExampleAppError::UrlParseError {
            source: e,
            port: config.grpc_port,
        })?;
    Ok(async move {
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
    })
}

#[tokio::main]
async fn main() -> Result<(), ExampleAppError> {
    let config = init_config()?;

    init_tracing(&config)?;

    info!("Starting example service...");

    init_metrics(&config)?;

    let store = init_store().await?;

    let (core, client_task) = init_core(store, &config)?;

    let http_server_task = init_http_interface(core.clone(), &config);

    let grpc_server_task = init_grpc_interface(core, &config)?;

    tokio::try_join!(http_server_task, grpc_server_task, client_task)?;

    Ok(())
}
