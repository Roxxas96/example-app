use std::{sync::Arc, time::Duration};

use crate::core::{Core, CoreError};
use crate::stores::hashmap::HashmapStoreError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{sync::Mutex, task::JoinHandle, time::sleep};
use tower_http::trace::TraceLayer;
use tracing::{info, trace};

#[derive(Error, Debug)]
pub enum HttpInterfaceError {
    #[error("Axum serve error")]
    AxumServe {
        #[source]
        source: std::io::Error,
        address: String,
    },
    #[error("Error creating the TCP listener with address {address:?}")]
    TcpListenerCreation {
        #[source]
        source: std::io::Error,
        address: String,
    },
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
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unknown error".to_string(),
            ),
        }
    }
}

pub struct HttpInterface {
    store: Arc<Mutex<Core>>,
}

impl HttpInterface {
    pub fn new(hashmap_store: Arc<Mutex<Core>>) -> Self {
        HttpInterface {
            store: hashmap_store,
        }
    }

    pub async fn start_app(&self, port: u16) -> JoinHandle<Result<(), HttpInterfaceError>> {
        let app = self.create_app();

        tokio::spawn(async move {
            let address = format!("0.0.0.0:{0}", port);
            let listener = tokio::net::TcpListener::bind(address.clone())
                .await
                .map_err(|e| HttpInterfaceError::TcpListenerCreation {
                    source: e,
                    address: address.clone(),
                })?;

            info!("Starting http interface on address {0}...", address);
            axum::serve(listener, app)
                .await
                .map_err(|e| HttpInterfaceError::AxumServe { source: e, address })
        })
    }

    fn create_app(&self) -> Router {
        Router::new()
            .route("/word", post(add_word).delete(remove_word))
            .route("/word/{word}", get(get_word))
            .route("/word/random", post(random_word))
            .with_state(self.store.clone())
            .route(
                "/health",
                get(|| async {
                    sleep(Duration::from_secs(5)).await;
                    "Ok"
                }),
            )
            .layer(TraceLayer::new_for_http())
    }
}

#[derive(Deserialize)]
struct AddWordRequest {
    pub word: String,
}

async fn add_word(
    State(state): State<Arc<Mutex<Core>>>,
    Json(payload): Json<AddWordRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    trace!("Received add_word request for word: {}", payload.word);
    state
        .lock()
        .await
        .add_word(payload.word.clone())
        .await
        .map_err(|err| match err {
            CoreError::HashmapStoreError(err) => match err {
                HashmapStoreError::AlreadyExists(word) => HttpInterfaceError::Conflict(word).into(),
                _ => HttpInterfaceError::InternalServerError.into(),
            },
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|_| StatusCode::CREATED)
}

#[derive(Serialize)]
struct GetWordResponse {
    pub word: String,
}

async fn get_word(
    State(state): State<Arc<Mutex<Core>>>,
    Path(word): Path<String>,
) -> Result<(StatusCode, Json<GetWordResponse>), (StatusCode, String)> {
    trace!("Received get_word request for word: {}", word);
    state
        .lock()
        .await
        .get_word(word.clone())
        .await
        .map_err(|err| match err {
            CoreError::HashmapStoreError(err) => match err {
                HashmapStoreError::NotFound(word) => HttpInterfaceError::NotFound(word).into(),
                _ => HttpInterfaceError::InternalServerError.into(),
            },
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|word| (StatusCode::OK, Json(GetWordResponse { word })))
}

#[derive(Deserialize)]
struct RemoveWordRequest {
    pub word: String,
}

async fn remove_word(
    State(state): State<Arc<Mutex<Core>>>,
    Json(payload): Json<RemoveWordRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    trace!("Received remove_word request for word: {}", payload.word);
    state
        .lock()
        .await
        .delete_word(payload.word.clone())
        .await
        .map_err(|err| match err {
            CoreError::HashmapStoreError(err) => match err {
                HashmapStoreError::NotFound(word) => {
                    HttpInterfaceError::BadRequest(format!("Word {:?} does not exists", word))
                        .into()
                }
                _ => HttpInterfaceError::InternalServerError.into(),
            },
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|_| StatusCode::OK)
}

#[derive(Serialize)]
struct RandomWordResponse {
    pub word: String,
}

async fn random_word(
    State(state): State<Arc<Mutex<Core>>>,
) -> Result<(StatusCode, Json<RandomWordResponse>), (StatusCode, String)> {
    trace!("Received random_word request");
    state
        .lock()
        .await
        .random_word()
        .await
        .map_err(|err| match err {
            CoreError::HashmapStoreError(err) => match err {
                HashmapStoreError::Empty => {
                    HttpInterfaceError::BadRequest("Store is empty".to_string()).into()
                }
                _ => HttpInterfaceError::InternalServerError.into(),
            },
            _ => HttpInterfaceError::InternalServerError.into(),
        })
        .map(|word| (StatusCode::OK, Json(RandomWordResponse { word })))
}
