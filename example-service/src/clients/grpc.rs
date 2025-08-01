use std::time::Duration;

use crate::clients::{Client, ClientError};
use async_recursion::async_recursion;
use axum::http::uri::InvalidUri;
use thiserror::Error;
use tonic::async_trait;
use tonic::transport::Channel;
use tonic_tracing_opentelemetry::middleware::client::{OtelGrpcLayer, OtelGrpcService};
use tower::ServiceBuilder;
use tracing::{trace, warn};
use word::{word_service_client::WordServiceClient, ChainRequest, HealthRequest};

pub mod word {
    tonic::include_proto!("word");
}

const MAX_RETRIES: u8 = 10;

#[derive(Error, Debug)]
pub enum GrpcClientError {
    #[error("Failed to connect to the server")]
    ConnectionError {
        #[source]
        source: tonic::transport::Error,
        address: String,
    },
    #[error("Invalid Uri when connecting to the server")]
    InvalidUri(#[source] InvalidUri),
}

#[derive(Clone, Debug)]
pub struct GrpcClient {
    client: WordServiceClient<OtelGrpcService<Channel>>,
    service_url: String,
}

impl GrpcClient {
    pub async fn new(service_url: String) -> Result<Self, GrpcClientError> {
        let client = connect_to_client(service_url.clone(), MAX_RETRIES).await?;

        Ok(GrpcClient {
            client,
            service_url,
        })
    }
}

#[async_trait]
impl Client for GrpcClient {
    type E = GrpcClientError;

    fn get_url(&self) -> String {
        self.service_url.clone()
    }

    async fn health(&mut self) -> Result<(), ClientError<GrpcClientError>> {
        self.client
            .health(HealthRequest {})
            .await
            .map_err(|_| ClientError::ServiceUnavailable)?;

        Ok(())
    }

    #[tracing::instrument(fields(component = "Grpc Client"), skip(self))]
    async fn chain(
        &mut self,
        word_chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, ClientError<GrpcClientError>> {
        let request = ChainRequest {
            input: word_chain,
            count,
        };
        trace!("Sending chain request: {:?}", request.clone());
        Ok(self
            .client
            .chain(request)
            .await
            .map_err(|status| match status.code() {
                tonic::Code::InvalidArgument => {
                    ClientError::BadRequest(status.message().to_string())
                }
                _ => ClientError::InternalServerError,
            })?
            .into_inner()
            .output)
    }
}

#[async_recursion]
async fn connect_to_client(
    service_url: String,
    retries: u8,
) -> Result<WordServiceClient<OtelGrpcService<Channel>>, GrpcClientError> {
    match Channel::from_shared(service_url.clone())
        .map_err(GrpcClientError::InvalidUri)?
        .connect()
        .await
    {
        Ok(channel) => Ok(WordServiceClient::new(
            ServiceBuilder::new().layer(OtelGrpcLayer).service(channel),
        )),
        Err(e) => {
            if retries > 0 {
                warn!(
                    "Failed to connect to server {0} : {1}. Retrying...",
                    service_url, e
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
                connect_to_client(service_url, retries - 1).await
            } else {
                Err(GrpcClientError::ConnectionError {
                    source: e,
                    address: service_url.clone(),
                })
            }
        }
    }
}
