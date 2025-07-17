use crate::clients::Client;
use crate::core::{Core, CoreError};
use crate::stores::Store;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower_http::trace::TraceLayer;
use tracing::{info, trace, warn};

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
    #[error("Service unavailable")]
    ServiceUnavailable,
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
            Self::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable".to_string(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unknown error".to_string(),
            ),
        }
    }
}

pub struct HttpInterface<S: Store, C: Client> {
    core: Core<S, C>,
}

impl<S: Store, C: Client> HttpInterface<S, C> {
    pub fn new(core: Core<S, C>) -> Self {
        HttpInterface { core }
    }

    pub async fn start_app(&self, port: u16) -> Result<(), HttpInterfaceError> {
        let app = self.create_app();

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
    }

    fn create_app(&self) -> Router {
        Router::new()
            .route("/word", post(Self::add_word).delete(Self::remove_word))
            .route("/word/{word}", get(Self::get_word))
            .route("/word/random", post(Self::random_word))
            .route("/word/chain", post(Self::start_chain))
            .route("/health", get(Self::health_check))
            .route("/ready", get(Self::ready_check))
            .with_state(self.core.clone())
            .layer(TraceLayer::new_for_http())
    }

    async fn health_check(State(state): State<Core<S, C>>) -> Result<(), (StatusCode, String)> {
        state.health_check().await.map_err(|err| match err {
            CoreError::ServiceUnavailable => HttpInterfaceError::ServiceUnavailable.into(),
            _ => HttpInterfaceError::InternalServerError.into(),
        })
    }

    async fn ready_check(State(state): State<Core<S, C>>) -> Result<(), (StatusCode, String)> {
        state.ready_check().await.map_err(|err| match err {
            _ => HttpInterfaceError::InternalServerError.into(),
        })
    }

    #[tracing::instrument(fields(component = "Http Interface", method = "add_word"))]
    async fn add_word(
        State(mut state): State<Core<S, C>>,
        Json(payload): Json<AddWordRequest>,
    ) -> Result<StatusCode, (StatusCode, String)> {
        trace!("Received add_word request for word: {}", payload.word);
        state
            .add_word(payload.word.clone())
            .await
            .map_err(|err| match err {
                CoreError::AlreadyExists(word) => HttpInterfaceError::Conflict(word).into(),
                _ => HttpInterfaceError::InternalServerError.into(),
            })
            .map(|_| StatusCode::CREATED)
    }

    #[tracing::instrument(fields(component = "Http Interface", method = "get_word"))]
    async fn get_word(
        State(state): State<Core<S, C>>,
        Path(word): Path<String>,
    ) -> Result<(StatusCode, Json<GetWordResponse>), (StatusCode, String)> {
        trace!("Received get_word request for word: {}", word);
        state
            .get_word(word.clone())
            .await
            .map_err(|err| match err {
                CoreError::NotFound(word) => HttpInterfaceError::NotFound(word).into(),
                _ => HttpInterfaceError::InternalServerError.into(),
            })
            .map(|word| (StatusCode::OK, Json(GetWordResponse { word })))
    }

    #[tracing::instrument(fields(component = "Http Interface", method = "remove_word"))]
    async fn remove_word(
        State(mut state): State<Core<S, C>>,
        Json(payload): Json<RemoveWordRequest>,
    ) -> Result<StatusCode, (StatusCode, String)> {
        trace!("Received remove_word request for word: {}", payload.word);
        state
            .delete_word(payload.word.clone())
            .await
            .map_err(|err| match err {
                CoreError::NotFound(word) => {
                    HttpInterfaceError::BadRequest(format!("Word {:?} does not exists", word))
                        .into()
                }
                _ => HttpInterfaceError::InternalServerError.into(),
            })
            .map(|_| StatusCode::OK)
    }

    #[tracing::instrument(fields(component = "Http Interface", method = "random_word"))]
    async fn random_word(
        State(state): State<Core<S, C>>,
    ) -> Result<(StatusCode, Json<RandomWordResponse>), (StatusCode, String)> {
        trace!("Received random_word request");
        state
            .random_word()
            .await
            .map_err(|err| match err {
                CoreError::Empty => {
                    HttpInterfaceError::BadRequest("Store is empty".to_string()).into()
                }
                _ => HttpInterfaceError::InternalServerError.into(),
            })
            .map(|word| (StatusCode::OK, Json(RandomWordResponse { word })))
    }

    #[tracing::instrument(fields(component = "Http Interface", method = "start_chain"))]
    async fn start_chain(
        State(state): State<Core<S, C>>,
        Json(payload): Json<ChainRequest>,
    ) -> Result<(StatusCode, Json<ChainResponse>), (StatusCode, String)> {
        trace!("Received chain request");
        // Implement the logic to generate a chain of words based on the inputs and count
        state
            .chain(payload.input, payload.count)
            .await
            .map_err(|err| match err {
                CoreError::NoConnectedServices => {
                    warn!("An attempt to chain was called but service is not connected to another example-service");
                    HttpInterfaceError::BadRequest(
                        "This service is not connected to another example-service".to_string(),
                    ).into()
                },
                CoreError::Empty => HttpInterfaceError::BadRequest("This service is not connected to another example-service".to_string()).into(),
                _ => HttpInterfaceError::InternalServerError.into(),
            })
            .map(|chain| Ok((StatusCode::OK, Json(ChainResponse { outputs: chain }))))?
    }
}

#[derive(Deserialize, Debug)]
struct ChainRequest {
    pub input: Vec<String>,
    pub count: u32,
}

#[derive(Serialize, Debug)]
struct ChainResponse {
    pub outputs: Vec<String>,
}

#[derive(Serialize, Debug)]
struct RandomWordResponse {
    pub word: String,
}

#[derive(Deserialize, Debug)]
struct RemoveWordRequest {
    pub word: String,
}

#[derive(Serialize, Debug)]
struct GetWordResponse {
    pub word: String,
}

#[derive(Deserialize, Debug)]
struct AddWordRequest {
    pub word: String,
}
