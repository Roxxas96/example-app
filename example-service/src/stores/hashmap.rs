use crate::stores::{Store, StoreError};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tonic::async_trait;
use tracing::trace;

#[derive(Error, Debug)]
pub enum HashmapStoreError {
    #[error("Wrong index generation during random picking")]
    WrongIndexGeneration,
}

#[derive(Clone)]
pub struct HashmapStore {
    pub word_store: Arc<RwLock<HashMap<String, String>>>,
}

impl HashmapStore {
    pub async fn new() -> Result<HashmapStore, HashmapStoreError> {
        let initial_store = Arc::new(RwLock::new(HashMap::new()));

        initial_store
            .write()
            .await
            .insert("hello".to_string(), "hello".to_string());
        initial_store
            .write()
            .await
            .insert("world".to_string(), "world".to_string());
        initial_store
            .write()
            .await
            .insert("how".to_string(), "how".to_string());
        initial_store
            .write()
            .await
            .insert("are".to_string(), "are".to_string());
        initial_store
            .write()
            .await
            .insert("you".to_string(), "you".to_string());
        initial_store
            .write()
            .await
            .insert("?".to_string(), "?".to_string());

        Ok(HashmapStore {
            word_store: initial_store,
        })
    }
}

#[async_trait]
impl Store for HashmapStore {
    type E = HashmapStoreError;

    async fn get_word(&self, word: String) -> Result<String, StoreError<HashmapStoreError>> {
        trace!("Getting word {:?} from hashmap store...", word);

        Ok(self
            .word_store
            .read()
            .await
            .get(&word)
            .ok_or(StoreError::NotFound(word))?
            .to_string())
    }

    async fn get_random_word(&self) -> Result<String, StoreError<HashmapStoreError>> {
        trace!("Getting a random word from hashmap store...");

        if self.word_store.read().await.is_empty() {
            return Err(StoreError::Empty);
        }

        let keys: Vec<_> = self.word_store.read().await.keys().cloned().collect();
        let index = rand::rng().random_range(0..keys.len());

        Ok(self
            .word_store
            .read()
            .await
            .keys()
            .nth(index)
            .ok_or(StoreError::InternalStoreError(
                HashmapStoreError::WrongIndexGeneration,
            ))?
            .to_string())
    }

    async fn add_word(&mut self, word: String) -> Result<(), StoreError<HashmapStoreError>> {
        trace!("Adding word {:?} to hashmap store...", word);

        if self.word_store.read().await.get(&word).is_some() {
            return Err(StoreError::AlreadyExists(word));
        }

        self.word_store.write().await.insert(word.clone(), word);

        Ok(())
    }

    async fn remove_word(&mut self, word: String) -> Result<(), StoreError<HashmapStoreError>> {
        trace!("Removing word {:?} from hashmap store...", word);

        if self.word_store.read().await.get(&word).is_none() {
            return Err(StoreError::NotFound(word));
        }

        self.word_store.write().await.remove(&word);

        Ok(())
    }
}
