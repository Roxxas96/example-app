use std::sync::Arc;

use rand::random_range;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::{
    clients::grpc::{GrpcClient, GrpcClientError},
    stores::hashmap::{HashmapStore, HashmapStoreError},
};

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Hashmap store error")]
    HashmapStoreError(#[source] HashmapStoreError),
    #[error("Failed to get mutable reference for gRPC client")]
    GrpcClientGetError,
    #[error("gRPC client error")]
    GrpcClientError(#[source] GrpcClientError),
    #[error("Index error when picking random element in Vec")]
    IndexError,
}

pub struct Core {
    store: HashmapStore,
    connected_services: Vec<String>,
}

impl Core {
    pub fn new(hashmap_store: HashmapStore, connected_services: Vec<String>) -> Self {
        Core {
            store: hashmap_store,
            connected_services,
        }
    }

    pub async fn get_word(&self, word: String) -> Result<String, CoreError> {
        info!("Getting word {0}...", word);
        Ok(self
            .store
            .get_word(word)
            .await
            .map_err(CoreError::HashmapStoreError)?)
    }

    pub async fn add_word(&mut self, word: String) -> Result<(), CoreError> {
        info!("Adding word {0}...", word);
        Ok(self
            .store
            .add_word(word)
            .await
            .map_err(CoreError::HashmapStoreError)?)
    }

    pub async fn delete_word(&mut self, word: String) -> Result<(), CoreError> {
        info!("Deleting word {0}...", word);
        Ok(self
            .store
            .remove_word(word)
            .await
            .map_err(CoreError::HashmapStoreError)?)
    }

    pub async fn random_word(&mut self) -> Result<String, CoreError> {
        info!("Getting random word...");

        let random_word = self
            .store
            .random_word()
            .await
            .map_err(CoreError::HashmapStoreError)?;

        debug!("Picked random word: {0}", random_word);

        Ok(random_word)
    }

    pub async fn chain(
        &mut self,
        chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, CoreError> {
        let random_word = self
            .store
            .random_word()
            .await
            .map_err(CoreError::HashmapStoreError)?;

        info!(
            "Adding word {0} to the chain... Remaining count: {1}",
            random_word, count
        );

        let mut new_chain = chain.clone();
        new_chain.push(random_word);

        if count > 0 {
            if !self.connected_services.is_empty() {
                let random_service = self
                    .connected_services
                    .get(random_range(0..self.connected_services.len()))
                    .ok_or(CoreError::IndexError)?;

                info!("Chaining with client: {:?}", random_service);

                let mut client = GrpcClient::new(random_service.to_string())
                    .await
                    .map_err(CoreError::GrpcClientError)?;

                new_chain = client
                    .chain(new_chain, count - 1)
                    .await
                    .map_err(CoreError::GrpcClientError)?;
            }
        }

        Ok(new_chain)
    }
}
