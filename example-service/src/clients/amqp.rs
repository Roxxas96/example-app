use amqprs::{
    channel::{BasicPublishArguments, Channel},
    BasicProperties, FieldTable,
};
use prost::Message;
use thiserror::Error;
use tonic::async_trait;
use tracing::{trace, Span};

use crate::clients::{Client, ClientError};

// Include the generated protobuf code
pub mod word_proto {
    tonic::include_proto!("word");
}

#[derive(Error, Debug)]
pub enum AmqpClientError {
    #[error("Failed to publish message")]
    FailedToPublish(#[source] amqprs::error::Error),
}

#[derive(Clone)]
pub struct AmqpClient {
    channel: Channel,
    exchange_name: String,
    routing_key: String,
}

impl AmqpClient {
    pub fn new(channel: Channel, exchange_name: String, routing_key: String) -> Self {
        AmqpClient {
            channel,
            exchange_name,
            routing_key,
        }
    }
}

#[async_trait]
impl Client for AmqpClient {
    type E = AmqpClientError;

    fn get_url(&self) -> String {
        self.channel.channel_id().to_string()
    }

    async fn health(&mut self) -> Result<(), ClientError<AmqpClientError>> {
        if !self.channel.is_connection_open() {
            return Err(ClientError::ServiceUnavailable);
        }

        Ok(())
    }

    #[tracing::instrument(fields(component = "Amqp Client"), skip(self))]
    async fn chain(
        &mut self,
        word_chain: Vec<String>,
        count: u32,
    ) -> Result<Vec<String>, ClientError<AmqpClientError>> {
        let chain_request = word_proto::ChainRequest {
            input: word_chain,
            count,
        };
        trace!("Sending chain request: {:?}", chain_request);

        let content = chain_request.encode_to_vec();

        let args = BasicPublishArguments::new(&self.exchange_name, &self.routing_key);
        let mut headers = FieldTable::new();
        headers.insert(
            "content-type".try_into().unwrap(),
            "application/x-protobuf".try_into().unwrap(),
        );
        headers.insert(
            "message-type".try_into().unwrap(),
            "ChainRequest".try_into().unwrap(),
        );

        let props = BasicProperties::default()
            .with_content_encoding("content_encoding")
            .with_content_type("application/x-protobuf")
            .with_expiration("100000")
            .with_message_type("ChainRequest")
            .with_persistence(true)
            .with_priority(1)
            .with_timestamp(1743000001)
            .with_headers(headers)
            .finish();

        self.channel
            .basic_publish(props, content, args)
            .await
            .map_err(|e| ClientError::InternalClientError(AmqpClientError::FailedToPublish(e)))?;

        Ok(vec![])
    }
}
