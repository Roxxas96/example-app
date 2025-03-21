use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, trace};

#[derive(Error, Debug)]
pub enum HashmapStoreError {
    #[error("Random decided to crash")]
    Random(),
    #[error("Word {0} not found")]
    NotFound(String),
    #[error("Word {0} already exists")]
    AlreadyExists(String),
}

pub struct HashmapStore {
    pub word_store: Arc<Mutex<HashMap<String, String>>>,
}

impl HashmapStore {
    pub fn new() -> Result<HashmapStore, HashmapStoreError> {
        debug!("Creating new hashmap store...");

        let mut initial_store: HashMap<String, String> = HashMap::new();

        initial_store.insert("hello".to_string(), "hello".to_string());
        initial_store.insert("world".to_string(), "world".to_string());
        initial_store.insert("how".to_string(), "how".to_string());
        initial_store.insert("are".to_string(), "are".to_string());
        initial_store.insert("you".to_string(), "you".to_string());
        initial_store.insert("?".to_string(), "?".to_string());

        if rand::random() {
            return Err(HashmapStoreError::Random());
        }

        Ok(HashmapStore {
            word_store: Arc::new(Mutex::new(initial_store)),
        })
    }

    pub async fn get_word(&self, word: String) -> Result<String, HashmapStoreError> {
        trace!("Getting word {:?} from hashmap store...", word);

        let store = self.word_store.lock().await;

        Ok(store
            .get(&word)
            .ok_or(HashmapStoreError::NotFound(word))?
            .to_string())
    }

    pub async fn add_word(&self, word: String) -> Result<(), HashmapStoreError> {
        trace!("Adding word {:?} to hashmap store...", word);

        let mut store = self.word_store.lock().await;

        if store.get(&word).is_some() {
            return Err(HashmapStoreError::AlreadyExists(word));
        }

        store.insert(word.clone(), word);

        Ok(())
    }

    pub async fn remove_word(&self, word: String) -> Result<(), HashmapStoreError> {
        trace!("Removing word {:?} from hashmap store...", word);

        let mut store = self.word_store.lock().await;

        if store.get(&word).is_none() {
            return Err(HashmapStoreError::NotFound(word));
        }

        store.remove(&word);

        Ok(())
    }
}
