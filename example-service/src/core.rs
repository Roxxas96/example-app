use crate::clients::{Client, ClientError};
use crate::stores::{Store, StoreError};
use rand::random_range;
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum CoreError<SE: Error, CE: Error> {
    #[error("Hashmap store error")]
    StoreError(#[source] StoreError<SE>),
    #[error("Word {0} not found")]
    NotFound(String),
    #[error("Word {0} already exists")]
    AlreadyExists(String),
    #[error("Store is empty")]
    Empty,
    #[error("Service is unhealthy")]
    ServiceUnavailable,
    #[error("gRPC client error")]
    ClientError(#[source] ClientError<CE>),
    #[error("Index error when picking random element in Vec")]
    IndexError,
    #[error("This service is not connected to another example-service")]
    NoConnectedServices,
}

#[derive(Clone, Debug)]
pub struct Core<S: Store, C: Client> {
    store: S,
    connected_services: Arc<RwLock<Vec<C>>>,
}

impl<S: Store, C: Client> Core<S, C> {
    pub fn new(store: S, connected_services: Arc<RwLock<Vec<C>>>) -> Self {
        Core {
            store,
            connected_services,
        }
    }

    pub async fn health_check(&self) -> Result<(), CoreError<S::E, C::E>> {
        let mut connected_services = self.connected_services.read().await.clone();

        for service in connected_services.iter_mut() {
            if let Err(e) = service.health().await {
                error!(
                    "Health check failed for connected service {:0}: {:?}",
                    service.get_url(),
                    e
                );
                return Err(CoreError::ServiceUnavailable);
            }
        }
        Ok(())
    }

    pub async fn ready_check(&self) -> Result<(), CoreError<S::E, C::E>> {
        Ok(())
    }

    #[tracing::instrument(fields(component = "Core"), skip(self))]
    pub async fn get_word(&self, word: String) -> Result<String, CoreError<S::E, C::E>> {
        info!(
            component = "Core",
            method = "get_word",
            monotonic_counter.num_call = 1_u64,
            "Getting word {0}...",
            word,
        );

        Ok(self
            .store
            .get_word(word.clone())
            .await
            .map_err(|err| match err {
                StoreError::NotFound(word) => CoreError::NotFound(word),
                _ => {
                    error!("Unanticipated error getting word {:0}: {:?}", word, err);
                    CoreError::StoreError(err)
                }
            })?)
    }

    #[tracing::instrument(fields(component = "Core"), skip(self))]
    pub async fn add_word(&mut self, word: String) -> Result<(), CoreError<S::E, C::E>> {
        info!(
            component = "Core",
            method = "add_word",
            monotonic_counter.num_call = 1_u64,
            "Adding word {0}...",
            word,
        );

        Ok(self
            .store
            .add_word(word.clone())
            .await
            .map_err(|err| match err {
                StoreError::AlreadyExists(word) => CoreError::AlreadyExists(word),
                _ => {
                    error!("Unanticipated error adding word {:0}: {:?}", word, err);
                    CoreError::StoreError(err)
                }
            })?)
    }

    #[tracing::instrument(fields(component = "Core"), skip(self))]
    pub async fn delete_word(&mut self, word: String) -> Result<(), CoreError<S::E, C::E>> {
        info!(
            component = "Core",
            method = "delete_word",
            monotonic_counter.num_call = 1_u64,
            "Deleting word {0}...",
            word,
        );

        Ok(self
            .store
            .remove_word(word.clone())
            .await
            .map_err(|err| match err {
                StoreError::NotFound(word) => CoreError::NotFound(word),
                _ => {
                    error!("Unanticipated error deleting word {:0}: {:?}", word, err);
                    CoreError::StoreError(err)
                }
            })?)
    }

    #[tracing::instrument(fields(component = "Core"), skip(self))]
    pub async fn random_word(&self) -> Result<String, CoreError<S::E, C::E>> {
        info!(
            component = "Core",
            method = "random_word",
            monotonic_counter.num_call = 1_u64,
            "Getting random word..."
        );

        let random_word = self.select_random_word().await?;

        debug!("Picked random word: {0}", random_word);

        Ok(random_word)
    }

    #[tracing::instrument(fields(component = "Core"), skip(self))]
    pub async fn chain(
        &self,
        chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, CoreError<S::E, C::E>> {
        let random_word = self.select_random_word().await?;
        info!(
            component = "Core",
            method = "chain",
            histogram.chain_count = count,
            monotonic_counter.num_call = 1_u64,
            "Adding word {0} to the chain...",
            random_word,
        );

        let mut new_chain = chain.clone();
        new_chain.push(random_word);

        if count > 0 {
            if self.connected_services.read().await.is_empty() {
                warn!("Chain was called because no services connected!");
                return Err(CoreError::NoConnectedServices);
            } else {
                new_chain = self.chain_with_random_service(new_chain, count).await?;
            }
        }

        Ok(new_chain)
    }

    #[tracing::instrument(fields(component = "Core"), skip(self))]
    async fn select_random_word(&self) -> Result<String, CoreError<S::E, C::E>> {
        Ok(self
            .store
            .get_random_word()
            .await
            .map_err(|err| match err {
                StoreError::Empty => CoreError::Empty,
                _ => {
                    error!("Unanticipated error getting random word: {:?}", err);
                    CoreError::StoreError(err)
                }
            })?)
    }

    #[tracing::instrument(fields(component = "Core"), skip(self))]
    async fn chain_with_random_service(
        &self,
        chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, CoreError<S::E, C::E>> {
        let mut connected_services = self.connected_services.read().await.clone();
        let num_connected_services = connected_services.len();

        let random_service = connected_services
            .get_mut(random_range(0..num_connected_services))
            .ok_or(CoreError::IndexError)
            .map_err(|err: CoreError<S::E, C::E>| {
                error!("Error picking random service in list: {:?}", err);
                CoreError::IndexError
            })?;

        info!("Chaining with client: {:?}", random_service.get_url());

        Ok(random_service
            .chain(chain, count - 1)
            .await
            .map_err(|err| {
                error!(
                    "Error chaining to client {:?}: {:?}",
                    random_service.get_url(),
                    err
                );
                CoreError::ClientError(err)
            })?)
    }
}
