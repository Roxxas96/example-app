use crate::core::{Core, CoreError};
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use tracing::trace;
use word::{word_service_server::WordService, ChainRequest, ChainResponse};
use word::{HealthRequest, HealthResponse};

pub mod word {
    tonic::include_proto!("word");
}

#[derive(Error, Debug)]
pub enum GrpcInterfaceError {
    #[error("Error serving gRPC")]
    GrpcServerError {
        #[source]
        source: tonic::transport::Error,
        address: SocketAddr,
    },
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error")]
    InternalServerError,
}

impl Into<tonic::Status> for GrpcInterfaceError {
    fn into(self) -> tonic::Status {
        match self {
            GrpcInterfaceError::BadRequest(msg) => tonic::Status::invalid_argument(msg),
            GrpcInterfaceError::InternalServerError => tonic::Status::internal("Internal error"),
            _ => tonic::Status::internal("Unknown error"),
        }
    }
}

pub struct GrpcInterface {
    core: Arc<Mutex<Core>>,
}

impl GrpcInterface {
    pub fn new(core: Arc<Mutex<Core>>) -> Self {
        GrpcInterface { core }
    }
}

#[tonic::async_trait]
impl WordService for GrpcInterface {
    async fn chain(
        &self,
        request: Request<ChainRequest>,
    ) -> Result<Response<ChainResponse>, tonic::Status> {
        trace!("Received chain request: {:?}", request);

        let message = request.into_inner();
        let new_chain = self
            .core
            .lock()
            .await
            .chain(message.input, message.count)
            .await
            .map_err(|e| match e {
                CoreError::HashmapStoreError(e) => tonic::Status::internal(e.to_string()),
                _ => tonic::Status::unknown("Unknown error"),
            })?;

        Ok(Response::new(ChainResponse { output: new_chain }))
    }

    async fn health(
        &self,
        request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        trace!("Received health request: {:?}", request);
        Ok(Response::new(HealthResponse {}))
    }
}
