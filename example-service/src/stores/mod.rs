use std::{error::Error, fmt::Debug};
use thiserror::Error;
use tonic::async_trait;

pub mod hashmap;

#[derive(Error, Debug)]
pub enum StoreError<E: Error> {
    #[error("Word {0} not found")]
    NotFound(String),
    #[error("Word {0} already exists")]
    AlreadyExists(String),
    #[error("Store is empty")]
    Empty,
    #[error("Internal store error")]
    InternalStoreError(#[source] E),
}

#[async_trait]
pub trait Store: Send + Sync + 'static + Clone + Debug {
    type E: Error;

    async fn get_word(&self, word: String) -> Result<String, StoreError<Self::E>>;
    async fn get_random_word(&self) -> Result<String, StoreError<Self::E>>;
    async fn add_word(&mut self, word: String) -> Result<(), StoreError<Self::E>>;
    async fn remove_word(&mut self, word: String) -> Result<(), StoreError<Self::E>>;
}
