use async_trait::async_trait;

pub mod apollo;
pub mod consul;
pub mod etcd;
pub mod rabbitmq;
pub mod redis;

/// TODO: better design
#[async_trait]
pub trait Middleware {
    type Client;
    type Error;

    async fn make_client(&self) -> Result<Self::Client, Self::Error>;
}
