use anyhow::Result;
use tonic::transport::Channel;
use word::{word_service_client::WordServiceClient, ChainRequest};

pub mod word {
    tonic::include_proto!("word");
}

pub struct GrpcClient {
    client: WordServiceClient<Channel>,
}

impl GrpcClient {
    pub async fn new(service_url: String) -> Self {
        GrpcClient {
            client: WordServiceClient::connect(service_url).await.unwrap(),
        }
    }

    pub async fn chain(&mut self, word_chain: Vec<String>, count: u32) -> Result<Vec<String>> {
        Ok(self
            .client
            .chain(ChainRequest {
                input: word_chain,
                count,
            })
            .await
            .unwrap()
            .into_inner()
            .output)
    }
}
