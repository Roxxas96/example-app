use std::collections::HashMap;

use rand::Rng;
use thiserror::Error;
use tracing::trace;

#[derive(Error, Debug)]
pub enum HashmapStoreError {
    #[error("Random decided to crash")]
    Random,
    #[error("Word {0} not found")]
    NotFound(String),
    #[error("Word {0} already exists")]
    AlreadyExists(String),
    #[error("Store is empty")]
    Empty,
    #[error("Wrong index generation during random picking")]
    WrongIndexGeneration,
}

pub struct HashmapStore {
    pub word_store: HashMap<String, String>,
}

impl HashmapStore {
    pub fn new() -> Result<HashmapStore, HashmapStoreError> {
        let mut initial_store: HashMap<String, String> = HashMap::new();

        initial_store.insert("hello".to_string(), "hello".to_string());
        initial_store.insert("world".to_string(), "world".to_string());
        initial_store.insert("how".to_string(), "how".to_string());
        initial_store.insert("are".to_string(), "are".to_string());
        initial_store.insert("you".to_string(), "you".to_string());
        initial_store.insert("?".to_string(), "?".to_string());

        if rand::random() {
            return Err(HashmapStoreError::Random);
        }

        Ok(HashmapStore {
            word_store: initial_store,
        })
    }

    pub async fn get_word(&self, word: String) -> Result<String, HashmapStoreError> {
        trace!("Getting word {:?} from hashmap store...", word);

        Ok(self
            .word_store
            .get(&word)
            .ok_or(HashmapStoreError::NotFound(word))?
            .to_string())
    }

    pub async fn add_word(&mut self, word: String) -> Result<(), HashmapStoreError> {
        trace!("Adding word {:?} to hashmap store...", word);

        if self.word_store.get(&word).is_some() {
            return Err(HashmapStoreError::AlreadyExists(word));
        }

        self.word_store.insert(word.clone(), word);

        Ok(())
    }

    pub async fn remove_word(&mut self, word: String) -> Result<(), HashmapStoreError> {
        trace!("Removing word {:?} from hashmap store...", word);

        if self.word_store.get(&word).is_none() {
            return Err(HashmapStoreError::NotFound(word));
        }

        self.word_store.remove(&word);

        Ok(())
    }

    pub async fn random_word(&mut self) -> Result<String, HashmapStoreError> {
        trace!("Getting a random word from hashmap store...");

        if self.word_store.is_empty() {
            return Err(HashmapStoreError::Empty);
        }

        let mut rng = rand::rng();
        let index = rng.random_range(0..self.word_store.len());

        Ok(self
            .word_store
            .keys()
            .nth(index)
            .ok_or(HashmapStoreError::WrongIndexGeneration)?
            .to_string())
    }
}
