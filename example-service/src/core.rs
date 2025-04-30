use crate::stores::hashmap::HashmapStoreError::WrongIndexGeneration;
use crate::{
    clients::grpc::{GrpcClient, GrpcClientError},
    stores::hashmap::{HashmapStore, HashmapStoreError},
};
use metrics::{counter, gauge};
use rand::random_range;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Hashmap store error")]
    HashmapStoreError(#[source] HashmapStoreError),
    #[error("Word {0} not found")]
    NotFound(String),
    #[error("Word {0} already exists")]
    AlreadyExists(String),
    #[error("Store is empty")]
    Empty,
    #[error("gRPC client error")]
    GrpcClientError(#[source] GrpcClientError),
    #[error("Index error when picking random element in Vec")]
    IndexError,
    #[error("This service is not connected to another example-service")]
    NoConnectedServices,
}

#[derive(Clone)]
pub struct Core {
    store: Arc<RwLock<HashmapStore>>,
    connected_services: Vec<String>,
}

impl Core {
    pub fn new(hashmap_store: Arc<RwLock<HashmapStore>>, connected_services: Vec<String>) -> Self {
        Core {
            store: hashmap_store,
            connected_services,
        }
    }

    pub async fn get_word(&self, word: String) -> Result<String, CoreError> {
        info!("Getting word {0}...", word);
        counter!("get_word_num_call").increment(1);
        Ok(self
            .store
            .read()
            .await
            .get_word(word)
            .await
            .map_err(|err| match err {
                HashmapStoreError::NotFound(word) => CoreError::NotFound(word),
                _ => CoreError::HashmapStoreError(err),
            })?)
    }

    pub async fn add_word(&mut self, word: String) -> Result<(), CoreError> {
        info!("Adding word {0}...", word);
        counter!("add_word_num_call").increment(1);
        Ok(self
            .store
            .write()
            .await
            .add_word(word)
            .await
            .map_err(|err| match err {
                HashmapStoreError::AlreadyExists(word) => CoreError::AlreadyExists(word),
                _ => CoreError::HashmapStoreError(err),
            })?)
    }

    pub async fn delete_word(&mut self, word: String) -> Result<(), CoreError> {
        info!("Deleting word {0}...", word);
        counter!("delete_word_num_call").increment(1);
        Ok(self
            .store
            .write()
            .await
            .remove_word(word)
            .await
            .map_err(|err| match err {
                HashmapStoreError::NotFound(word) => CoreError::NotFound(word),
                _ => CoreError::HashmapStoreError(err),
            })?)
    }

    pub async fn random_word(&self) -> Result<String, CoreError> {
        info!("Getting random word...");
        counter!("random_word_num_call").increment(1);

        let random_word = self.select_random_word().await?;

        debug!("Picked random word: {0}", random_word);

        Ok(random_word)
    }

    pub async fn chain(&self, chain: Vec<String>, count: u32) -> Result<Vec<String>, CoreError> {
        counter!("chain_word_num_call").increment(1);
        gauge!("chain_word_count").set(count);
        let random_word = self.select_random_word().await?;

        info!(
            "Adding word {0} to the chain... Remaining count: {1}",
            random_word, count
        );

        let mut new_chain = chain.clone();
        new_chain.push(random_word);

        if count > 0 {
            if self.connected_services.is_empty() {
                return Err(CoreError::NoConnectedServices);
            } else {
                new_chain = self.chain_with_random_service(new_chain, count).await?;
            }
        }

        Ok(new_chain)
    }

    async fn select_random_word(&self) -> Result<String, CoreError> {
        Ok(self
            .store
            .read()
            .await
            .random_word()
            .await
            .map_err(|err| match err {
                WrongIndexGeneration => {
                    error!("Error during selection of random word: {:?}", err);
                    CoreError::HashmapStoreError(err)
                }
                HashmapStoreError::Empty => CoreError::Empty,
                _ => CoreError::HashmapStoreError(err),
            })?)
    }

    async fn chain_with_random_service(
        &self,
        chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, CoreError> {
        let random_service = self
            .connected_services
            .get(random_range(0..self.connected_services.len()))
            .ok_or(CoreError::IndexError)
            .map_err(|err| match err {
                _ => {
                    error!("Error picking random service in list: {:?}", err);
                    CoreError::IndexError
                }
            })?;

        info!("Chaining with client: {:?}", random_service);

        let mut client =
            GrpcClient::new(random_service.to_string())
                .await
                .map_err(|err| match err {
                    _ => {
                        error!("Error connecting to client {:?}: {:?}", random_service, err);
                        CoreError::GrpcClientError(err)
                    }
                })?;

        Ok(client
            .chain(chain, count - 1)
            .await
            .map_err(|err| match err {
                _ => {
                    error!("Error chaining to client {:?}: {:?}", random_service, err);
                    CoreError::GrpcClientError(err)
                }
            })?)
    }
}
