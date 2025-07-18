use crate::clients::Client;
use crate::core::{Core, CoreError};
use crate::stores::Store;
use std::net::SocketAddr;
use thiserror::Error;
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

impl Into<Status> for GrpcInterfaceError {
    fn into(self) -> Status {
        match self {
            GrpcInterfaceError::BadRequest(msg) => Status::invalid_argument(msg),
            GrpcInterfaceError::InternalServerError => Status::internal("Internal error"),
            _ => Status::internal("Unknown error"),
        }
    }
}

#[derive(Debug)]
pub struct GrpcInterface<S: Store, C: Client> {
    core: Core<S, C>,
}

impl<S: Store, C: Client> GrpcInterface<S, C> {
    pub fn new(core: Core<S, C>) -> Self {
        GrpcInterface { core }
    }
}

#[tonic::async_trait]
impl<S: Store, C: Client> WordService for GrpcInterface<S, C> {
    #[tracing::instrument(fields(component = "Grpc Interface"), skip(self))]
    async fn chain(
        &self,
        request: Request<ChainRequest>,
    ) -> Result<Response<ChainResponse>, Status> {
        trace!("Received chain request: {:?}", request);

        let message = request.into_inner();
        let new_chain = self
            .core
            .chain(message.input, message.count)
            .await
            .map_err(|e| match e {
                CoreError::NoConnectedServices => {
                    <GrpcInterfaceError as Into<Status>>::into(GrpcInterfaceError::BadRequest(
                        "This service is not connected to an example-service".to_string(),
                    ))
                }
                CoreError::Empty => <GrpcInterfaceError as Into<Status>>::into(
                    GrpcInterfaceError::BadRequest("The store is empty".to_string()),
                ),
                _ => GrpcInterfaceError::InternalServerError.into(),
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
