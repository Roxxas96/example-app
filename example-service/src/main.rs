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
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::{net::AddrParseError, sync::Arc};
use stores::hashmap::{HashmapStore, HashmapStoreError};
use thiserror::Error;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{debug, error, info};
use tracing_opentelemetry::MetricsLayer;
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
    #[error("Error when building OpenTelemetry span exporter")]
    SpanExporterBuildError(#[source] opentelemetry_otlp::ExporterBuildError),
    #[error("Error when building OpenTelemetry metrics exporter")]
    MetricsExporterBuildError(#[source] opentelemetry_otlp::ExporterBuildError),
    #[error("Error when init tracing registry")]
    TracingRegistryInitError(#[source] tracing_subscriber::util::TryInitError),
}

#[derive(Serialize, Deserialize, Debug)]
struct ExampleAppConfig {
    http_port: u16,
    grpc_port: u16,
    connected_services: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MonitoringConfig {
    metrics_push_interval: u64,
}

fn init_config() -> Result<(ExampleAppConfig, MonitoringConfig), ExampleAppError> {
    let app_config = Config::builder()
        .add_source(config::Environment::default().prefix("EXAMPLE_SERVICE"))
        .set_default("http_port", 3001)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("grpc_port", 50051)
        .map_err(ExampleAppError::ConfigError)?
        .set_default("connected_services", "")
        .map_err(ExampleAppError::ConfigError)?
        .build()
        .map_err(ExampleAppError::ConfigError)?
        .try_deserialize()
        .map_err(ExampleAppError::ConfigError)?;

    let monitoring_config = Config::builder()
        .add_source(config::Environment::default().prefix("MONITORING"))
        .set_default("metrics_push_interval", 5)
        .map_err(ExampleAppError::ConfigError)?
        .build()
        .map_err(ExampleAppError::ConfigError)?
        .try_deserialize()
        .map_err(ExampleAppError::ConfigError)?;

    Ok((app_config, monitoring_config))
}

fn init_tracer_provider() -> Result<SdkTracerProvider, ExampleAppError> {
    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .build()
                .map_err(ExampleAppError::SpanExporterBuildError)?,
        )
        .build())
}

fn init_meter_provider(config: &MonitoringConfig) -> Result<SdkMeterProvider, ExampleAppError> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()
        .map_err(ExampleAppError::MetricsExporterBuildError)?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(
            config.metrics_push_interval.clone(),
        ))
        .build();

    let meter_provider = SdkMeterProvider::builder().with_reader(reader).build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}

fn init_tracing(config: &MonitoringConfig) -> Result<(), ExampleAppError> {
    let tracer_provider = init_tracer_provider()?;
    let tracer = tracer_provider.tracer("readme_example");

    let meter_provider = init_meter_provider(config)?;

    Registry::default()
        .with(tracing_subscriber::filter::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::Layer::default())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(MetricsLayer::new(meter_provider))
        .try_init()
        .map_err(ExampleAppError::TracingRegistryInitError)?;
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
            if !urls.is_empty() {
                for service_url in urls.split(',') {
                    let client = GrpcClient::new(service_url.to_string())
                        .await
                        .map_err(ExampleAppError::GrpcClientError)?;
                    grpc_clients_clone.write().await.push(client);
                    debug!("Connected gRPC client to {:?}", service_url);
                }
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
    let (app_config, monitoring_config) = init_config()?;

    init_tracing(&monitoring_config)?;

    info!("Starting example service...");

    let store = init_store().await?;

    let (core, client_task) = init_core(store, &app_config)?;

    let http_server_task = init_http_interface(core.clone(), &app_config);

    let grpc_server_task = init_grpc_interface(core, &app_config)?;

    tokio::try_join!(http_server_task, grpc_server_task, client_task)?;

    Ok(())
}
