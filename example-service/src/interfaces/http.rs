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
use tokio::{sync::Mutex, task::JoinHandle};
use tower_http::trace::TraceLayer;
use tracing::debug;

use crate::stores::hashmap::{HashmapStore, HashmapStoreError};

#[derive(Error, Debug)]
pub enum HttpInterfaceError {
    #[error("Axum serve error: {0}")]
    AxumServe(std::io::Error),
    #[error("Word {0} not found")]
    NotFound(String),
    #[error("Word {0} already exists")]
    Conflict(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error")]
    InternalServerError,
}

impl Into<(StatusCode, String)> for HttpInterfaceError {
    fn into(self) -> (StatusCode, String) {
        match self {
            Self::AxumServe(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Self::NotFound(word) => (StatusCode::NOT_FOUND, format!("Word '{}' not found", word)),
            Self::Conflict(word) => (
                StatusCode::CONFLICT,
                format!("Word '{}' already exists", word),
            ),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        }
    }
}

pub struct HttpInterface {
    store: Arc<Mutex<HashmapStore>>,
}

impl HttpInterface {
    pub fn new(hashmap_store: Arc<Mutex<HashmapStore>>) -> Self {
        HttpInterface {
            store: hashmap_store,
        }
    }

    pub async fn start_app(&self) -> JoinHandle<Result<(), HttpInterfaceError>> {
        debug!("Starting HTTP interface on port 3000...");

        let app = self.create_app();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .map_err(HttpInterfaceError::AxumServe)
        })
    }

    fn create_app(&self) -> Router {
        return Router::new()
            .route("/word", post(add_word).delete(remove_word))
            .route("/word/{word}", get(get_word))
            .route("/word/random", post(random_word))
            .with_state(self.store.clone())
            .route("/health", get(|| async { "Ok" }))
            .layer(TraceLayer::new_for_http());
    }
}

#[derive(Deserialize)]
struct AddWordRequest {
    pub word: String,
}

async fn add_word(
    State(state): State<Arc<Mutex<HashmapStore>>>,
    Json(payload): Json<AddWordRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .lock()
        .await
        .add_word(payload.word.clone())
        .await
        .map_err(|err| match err {
            HashmapStoreError::AlreadyExists(word) => HttpInterfaceError::Conflict(word).into(),
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|_| StatusCode::CREATED)
}

#[derive(Serialize)]
struct GetWordResponse {
    pub word: String,
}

async fn get_word(
    State(state): State<Arc<Mutex<HashmapStore>>>,
    Path(word): Path<String>,
) -> Result<(StatusCode, Json<GetWordResponse>), (StatusCode, String)> {
    state
        .lock()
        .await
        .get_word(word.clone())
        .await
        .map_err(|err| match err {
            HashmapStoreError::NotFound(word) => HttpInterfaceError::NotFound(word).into(),
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|word| (StatusCode::OK, Json(GetWordResponse { word })))
}

#[derive(Deserialize)]
struct RemoveWordRequest {
    pub word: String,
}

async fn remove_word(
    State(state): State<Arc<Mutex<HashmapStore>>>,
    Json(payload): Json<RemoveWordRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .lock()
        .await
        .remove_word(payload.word.clone())
        .await
        .map_err(|err| match err {
            HashmapStoreError::NotFound(word) => {
                HttpInterfaceError::BadRequest(format!("Word {:?} does not exists", word)).into()
            }
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|_| StatusCode::OK)
}

#[derive(Serialize)]
struct RandomWordResponse {
    pub word: String,
}

async fn random_word(
    State(state): State<Arc<Mutex<HashmapStore>>>,
) -> Result<(StatusCode, Json<RandomWordResponse>), (StatusCode, String)> {
    state
        .lock()
        .await
        .random_word()
        .await
        .map_err(|err| match err {
            HashmapStoreError::Empty => {
                HttpInterfaceError::BadRequest("Store is empty".to_string()).into()
            }
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|word| (StatusCode::OK, Json(RandomWordResponse { word })))
}
