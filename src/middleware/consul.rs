use crate::config::env::{optional, optional_some};
use crate::define_config;
use crate::middleware::Middleware;
use async_trait::async_trait;
use serde::Serialize;

define_config! {
    #[derive(Serialize, Debug)]
    pub ConsulConf {
        #[default_addr = "default_addr"]
        pub addr -> String {
            optional("CONSUL_HTTP_ADDR", "http://127.0.0.1:8500")
        },
        #[default_token = "default_token"]
        pub token -> Option<String> {
            optional_some("CONSUL_HTTP_TOKEN")
        }
    }
}

#[derive(Clone)]
pub struct Consul(ConsulConf);

impl Consul {
    pub fn new(conf: ConsulConf) -> Self {
        Self(conf)
    }
}

#[async_trait]
impl Middleware for Consul {
    type Client = consul::Client;
    type Error = consul::errors::Error;

    async fn make_client(&self) -> Result<Self::Client, Self::Error> {
        let conf = consul::Config::new_from_addr(&self.0.addr, self.0.token.clone())?;
        Ok(consul::Client::new(conf))
    }
}
