use crate::config::env::{optional, optional_some, require};
use crate::define_config;
use crate::middleware::{parse_config_type, Middleware};
use async_trait::async_trait;
use kosei::ApolloClient;
use serde::Serialize;
use std::convert::Infallible;

define_config! {
    #[derive(Serialize, Debug)]
    pub ApolloConf {
        #[default_addr = "default_addr"]
        pub addr -> String {
            require("APOLLO_ADDR")
        },
        #[default_appid = "default_appid"]
        pub appid -> String {
            require("APOLLO_APPID")
        },
        #[default_namespace = "default_namespace"]
        pub namespace -> String {
            require("APOLLO_NS")
        },
        #[default_config_type = "default_config_type"]
        pub config_type -> String {
            optional("APOLLO_CONFIG_TYPE", "yaml")
        },
        #[default_cluster_name = "default_cluster_name"]
        pub cluster_name -> String {
            optional("APOLLO_CLUSTER_NAME", "default")
        },
        #[default_secret = "default_secret"]
        pub secret -> Option<String> {
            optional_some("APOLLO_SECRET")
        }
    }
}

pub struct Apollo(ApolloConf);

impl Apollo {
    pub fn new(conf: ApolloConf) -> Self {
        Self(conf)
    }
}

#[async_trait]
impl Middleware for Apollo {
    type Client = ApolloClient;
    type Error = Infallible;

    async fn make_client(&self) -> Result<Self::Client, Self::Error> {
        let conf = &self.0;
        let mut client = ApolloClient::new(&conf.addr)
            .appid(&conf.appid)
            .cluster(&conf.cluster_name)
            .namespace(&conf.namespace, parse_config_type(&conf.config_type));
        if let Some(ref secret) = self.0.secret {
            client = client.secret(secret);
        }
        Ok(client)
    }
}
