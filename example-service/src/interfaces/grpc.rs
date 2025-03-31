use std::sync::Arc;

use anyhow::Result;
use rand::random_range;
use thiserror::Error;
use tokio::sync::Mutex;
use tonic::{Request, Response};
use word::{word_service_server::WordService, ChainRequest, ChainResponse};

use crate::{
    clients::grpc::GrpcClient,
    stores::hashmap::{HashmapStore, HashmapStoreError},
};

pub mod word {
    tonic::include_proto!("word");
}

#[derive(Error, Debug)]
pub enum GrpcInterfaceError {
    #[error("Error serving gRPC: {0}")]
    GrpcServerError(tonic::transport::Error),
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
    store: Arc<Mutex<HashmapStore>>,
    clients: Arc<Mutex<Vec<GrpcClient>>>,
}

impl GrpcInterface {
    pub fn new(
        hashmap_store: Arc<Mutex<HashmapStore>>,
        grpc_client: Arc<Mutex<Vec<GrpcClient>>>,
    ) -> Self {
        GrpcInterface {
            store: hashmap_store,
            clients: grpc_client,
        }
    }
}

#[tonic::async_trait]
impl WordService for GrpcInterface {
    async fn chain(
        &self,
        request: Request<ChainRequest>,
    ) -> Result<Response<ChainResponse>, tonic::Status> {
        let random_word =
            self.store
                .lock()
                .await
                .random_word()
                .await
                .map_err(|err| -> tonic::Status {
                    match err {
                        HashmapStoreError::Empty => {
                            GrpcInterfaceError::BadRequest("Store is empty".to_string()).into()
                        }
                        _ => GrpcInterfaceError::InternalServerError.into(),
                    }
                })?;

        let message = request.into_inner();
        let mut new_chain = message.input.clone();
        new_chain.push(random_word);

        if message.count > 0 {
            let mut clients = self.clients.lock().await;
            if !clients.is_empty() {
                let random_client = random_range(0..clients.len());

                new_chain = clients
                    .get_mut(random_client)
                    .ok_or_else(|| -> tonic::Status {
                        GrpcInterfaceError::InternalServerError.into()
                    })?
                    .chain(new_chain, message.count - 1)
                    .await
                    .map_err(|_| -> tonic::Status {
                        GrpcInterfaceError::InternalServerError.into()
                    })?;
            }
        }

        Ok(Response::new(ChainResponse { output: new_chain }))
    }
}
