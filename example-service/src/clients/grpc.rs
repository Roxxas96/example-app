use std::time::Duration;

use async_recursion::async_recursion;
use thiserror::Error;
use tonic::transport::Channel;
use tracing::trace;
use word::{word_service_client::WordServiceClient, ChainRequest};

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
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error")]
    InternalServerError,
}

pub struct GrpcClient {
    client: WordServiceClient<Channel>,
}

impl GrpcClient {
    pub async fn new(service_url: String) -> Result<Self, GrpcClientError> {
        let client = connect_to_client(service_url, MAX_RETRIES).await?;

        Ok(GrpcClient { client })
    }

    pub async fn chain(
        &mut self,
        word_chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, GrpcClientError> {
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
                    GrpcClientError::BadRequest(status.message().to_string())
                }
                _ => GrpcClientError::InternalServerError,
            })?
            .into_inner()
            .output)
    }
}

#[async_recursion]
async fn connect_to_client(
    service_url: String,
    retries: u8,
) -> Result<WordServiceClient<Channel>, GrpcClientError> {
    match WordServiceClient::connect(service_url.clone()).await {
        Ok(client) => Ok(client),
        Err(e) => {
            if retries > 0 {
                tracing::warn!("Failed to connect to the server: {0}. Retrying...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                return connect_to_client(service_url, retries - 1).await;
            } else {
                Err(GrpcClientError::ConnectionError {
                    source: e,
                    address: service_url.clone(),
                })
            }
        }
    }
}
