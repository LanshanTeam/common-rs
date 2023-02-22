use async_trait::async_trait;
use kosei::ConfigType;

pub mod apollo;
pub mod consul;
pub mod etcd;
pub mod nacos;
pub mod rabbitmq;
pub mod redis;

/// TODO: better design
#[async_trait]
pub trait Middleware {
    type Client;
    type Error;

    async fn make_client(&self) -> Result<Self::Client, Self::Error>;
}

#[inline]
fn parse_config_type(typ: &str) -> ConfigType {
    match &*typ.to_lowercase() {
        "toml" => ConfigType::TOML,
        "json" => ConfigType::JSON,
        _ => ConfigType::YAML,
    }
}
