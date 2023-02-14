use crate::config::env::optional;
use crate::define_config;
use crate::middleware::Middleware;
use amqprs::connection::OpenConnectionArguments;
use async_trait::async_trait;
use serde::Serialize;

define_config! {
    #[derive(Serialize, Debug)]
    pub RabbitMQConf {
        #[default_host = "default_host"]
        pub enpoint -> String {
            optional("RABBITMQ_ENDPOINT", "guest:guest@localhost:5672")
        },
        #[default_virtual = "default_virtual"]
        pub virtual_host -> String {
            String::from("/")
        },
        #[default_heartbeat = "default_heartbeat"]
        pub heartbeat -> u16 {
            60
        }
    }
}

pub struct RabbitMQ(RabbitMQConf);

impl RabbitMQ {
    pub fn new(conf: RabbitMQConf) -> Self {
        Self(conf)
    }
}

#[async_trait]
impl Middleware for RabbitMQ {
    type Client = amqprs::connection::Connection;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn make_client(&self) -> Result<Self::Client, Self::Error> {
        let url = url::Url::parse(&self.0.enpoint)?;
        let arg = OpenConnectionArguments::new(
            url.host_str().unwrap_or("localhost"),
            url.port().unwrap_or(5672),
            url.username(),
            url.password().unwrap_or(""),
        )
        .virtual_host(&self.0.virtual_host)
        .heartbeat(self.0.heartbeat)
        .finish();
        let conn = amqprs::connection::Connection::open(&arg).await?;
        Ok(conn)
    }
}
