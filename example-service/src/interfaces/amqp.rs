use std::string::FromUtf8Error;

use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicConsumeArguments, Channel, QueueBindArguments,
        QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};
use opentelemetry::Context;
use prost::Message;
use thiserror::Error;
use tonic::async_trait;
use tracing::{error, info, instrument, trace, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{
    clients::Client,
    core::{Core, CoreError},
    stores::Store,
};

// Include the generated protobuf code
pub mod word_proto {
    tonic::include_proto!("word");
}

#[derive(Error, Debug)]
pub enum AmqpInterfaceError {
    #[error("Failed to connect to RabbitMQ server")]
    ConnectionError(#[source] amqprs::error::Error),
    #[error("Failed to declare queue")]
    QueueDeclareError,
    #[error("Failed parsing message content")]
    ParseMessageContentError(#[source] FromUtf8Error),
}

pub struct AmqpInterface<S: Store, C: Client> {
    core: Core<S, C>,
}

impl<S: Store, C: Client> AmqpInterface<S, C> {
    pub fn new(core: Core<S, C>) -> Self {
        AmqpInterface { core }
    }

    pub async fn register_consumer(
        &self,
        channel: Channel,
        queue_name: &str,
    ) -> Result<(), AmqpInterfaceError> {
        let args = BasicConsumeArguments::new(queue_name, "chain_consumer");

        channel
            .basic_consume(
                ChainConsumer {
                    core: self.core.clone(),
                },
                args,
            )
            .await
            .map_err(AmqpInterfaceError::ConnectionError)?;

        Ok(())
    }
}

#[derive(Debug)]
struct ChainConsumer<S: Store, C: Client> {
    core: Core<S, C>,
}

#[async_trait]
impl<S: Store, C: Client> AsyncConsumer for ChainConsumer<S, C> {
    #[tracing::instrument(fields(component = "Amqp Interface"), skip(self, channel, content))]
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        if let Ok(message) = word_proto::ChainRequest::decode(&*content) {
            trace!("Received chain request: {:?}", message);

            match self.core.chain(message.input, message.count).await {
                Err(CoreError::NoConnectedServices) => {
                    error!("This service is not connected to an example-service");
                }
                Err(CoreError::Empty) => {
                    error!("The store is empty");
                }
                Err(e) => error!("Internal error when chaining: {:?}", e),
                Ok(_) => (),
            }

            let args = BasicAckArguments::new(deliver.delivery_tag(), false);
            match channel
                .basic_ack(args)
                .await
                .map_err(AmqpInterfaceError::ConnectionError)
            {
                Ok(_) => (),
                Err(e) => error!("Failed to acknowledge message: {:?}", e),
            };
        } else {
            error!("Failed to deserialize protobuf message");
        }
    }
}
