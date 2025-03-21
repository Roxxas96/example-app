use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinHandle;

use crate::stores::hashmap::{HashmapStore, HashmapStoreError};

#[derive(Error, Debug)]
pub enum HttpInterfaceError {
    #[error("Axum serve error: {0}")]
    AxumServe(std::io::Error),
}

struct HttpInterfaceAppState {
    pub hashmap_store: HashmapStore,
}

pub struct HttpInterface {}

impl HttpInterface {
    pub async fn start_app(
        &self,
        hashmap_store: HashmapStore,
    ) -> JoinHandle<Result<(), HttpInterfaceError>> {
        let app = self.create_app(hashmap_store);
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .map_err(HttpInterfaceError::AxumServe)
        })
    }

    fn create_app(&self, hashmap_store: HashmapStore) -> Router {
        return Router::new()
            .route("/word", post(add_word).delete(remove_word))
            .route("/word/{word}", get(get_word))
            .with_state(Arc::new(HttpInterfaceAppState {
                hashmap_store: hashmap_store,
            }))
            .route("/health", get(|| async { "Ok" }));
    }
}

#[derive(Deserialize)]
struct AddWordRequest {
    pub word: String,
}

async fn add_word(
    State(state): State<Arc<HttpInterfaceAppState>>,
    Json(payload): Json<AddWordRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.hashmap_store.add_word(payload.word.clone()).await {
        Err(err) => match err {
            HashmapStoreError::AlreadyExists(_) => Err((
                StatusCode::CONFLICT,
                format!("Word {:?} already exists", payload.word),
            )),
            _ => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
            )),
        },
        Ok(_) => Ok(StatusCode::CREATED),
    }
}

#[derive(Serialize)]
struct GetWordResponse {
    pub word: String,
}

async fn get_word(
    State(state): State<Arc<HttpInterfaceAppState>>,
    Path(word): Path<String>,
) -> Result<(StatusCode, Json<GetWordResponse>), (StatusCode, String)> {
    match state.hashmap_store.get_word(word.clone()).await {
        Err(err) => match err {
            HashmapStoreError::NotFound(_) => {
                Err((StatusCode::NOT_FOUND, format!("Word {:?} not found", word)))
            }
            _ => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
            )),
        },
        Ok(val) => Ok((StatusCode::OK, Json(GetWordResponse { word: val }))),
    }
}

#[derive(Deserialize)]
struct RemoveWordRequest {
    pub word: String,
}

async fn remove_word(
    State(state): State<Arc<HttpInterfaceAppState>>,
    Json(payload): Json<RemoveWordRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.hashmap_store.remove_word(payload.word.clone()).await {
        Err(err) => match err {
            HashmapStoreError::NotFound(_) => Err((
                StatusCode::BAD_REQUEST,
                format!("Word {:?} not found", payload.word),
            )),
            _ => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
            )),
        },
        Ok(_) => Ok(StatusCode::OK),
    }
}
