use crate::config::env::{optional, optional_some, require};
use crate::define_config;
use crate::middleware::{parse_config_type, Middleware};
use async_trait::async_trait;
use kosei::nacos::{Builder, NacosClient};
use serde::Serialize;
use std::convert::Infallible;

define_config! {
    #[derive(Serialize, Debug)]
    pub NacosConf {
        #[default_addr = "default_addr"]
        pub addr -> String {
            require("NACOS_ADDR")
        },
        #[default_data_id = "default_data_id"]
        pub data_id -> String {
            require("NACOS_DATA_ID")
        },
        #[default_group = "default_group"]
        pub group -> String {
            optional("NACOS_GROUP", "DEFAULT_GROUP")
        },
        #[default_config_type = "default_config_type"]
        pub config_type -> String {
            optional("NACOS_CONFIG_TYPE", "yaml")
        },
        #[default_credential = "default_credential"]
        pub credential -> Option<[String; 2]> {
            optional_some("NACOS_CREDENTIAL").map(|v| {
                v.split(':')
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("environment 'NACOS_CREDENTIAL' must be like '[username]:[password]'")
            })
        }
    }
}

pub struct Nacos(NacosConf);

impl Nacos {
    pub fn new(conf: NacosConf) -> Self {
        Self(conf)
    }
}

#[async_trait]
impl Middleware for Nacos {
    type Client = NacosClient;
    type Error = Infallible;

    async fn make_client(&self) -> Result<Self::Client, Self::Error> {
        let mut builder = Builder::new()
            .server_url(self.0.addr.as_str())
            .data_id(self.0.data_id.as_str())
            .group(self.0.group.as_str())
            .config_type(parse_config_type(self.0.config_type.as_str()));
        if let Some(ref credential) = self.0.credential {
            builder = builder.credential(&credential[0], &credential[1]);
        }
        Ok(builder.finish())
    }
}
