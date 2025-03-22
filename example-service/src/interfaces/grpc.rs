use std::sync::Arc;

use anyhow::Result;
use rand::{random, random_range};
use tokio::sync::Mutex;
use tonic::{Request, Response};
use tracing::debug;
use word::{word_service_server::WordService, ChainRequest, ChainResponse};

use crate::{clients::grpc::GrpcClient, stores::hashmap::HashmapStore};

pub mod word {
    tonic::include_proto!("word");
}

pub struct GrpcInterface {
    store: Arc<Mutex<HashmapStore>>,
    clients: Vec<Arc<Mutex<GrpcClient>>>,
}

impl GrpcInterface {
    pub fn new(
        hashmap_store: Arc<Mutex<HashmapStore>>,
        grpc_client: Vec<Arc<Mutex<GrpcClient>>>,
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
        let random_word = self.store.lock().await.random_word().await.unwrap();
        let message = request.into_inner();
        let mut new_chain = message.input.clone();
        new_chain.push(random_word);

        if message.count > 0 && self.clients.len() > 0 {
            new_chain = self
                .clients
                .get(random_range(0..self.clients.len()))
                .unwrap()
                .lock()
                .await
                .chain(new_chain, message.count - 1)
                .await
                .unwrap();
        }

        Ok(Response::new(ChainResponse { output: new_chain }))
    }
}
