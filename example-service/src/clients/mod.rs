use std::{error::Error, fmt::Debug};
use thiserror::Error;
use tonic::async_trait;

pub mod amqp;
pub mod grpc;

#[derive(Error, Debug)]
pub enum ClientError<E: Error> {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Internal client error: {0}")]
    InternalClientError(#[source] E),
    #[error("Internal server error")]
    InternalServerError,
}

#[async_trait]
pub trait Client: Clone + Send + Sync + 'static {
    type E: Error;

    fn get_url(&self) -> String;

    async fn health(&mut self) -> Result<(), ClientError<Self::E>>;

    async fn chain(
        &mut self,
        word_chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, ClientError<Self::E>>;
}
