use crate::config::service::ServiceConf;
use crate::middleware::consul::{Consul, ConsulConf};
use crate::middleware::Middleware;
use crate::registry::{ConsulRegistryOption, ServiceRegister};
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct ConsulRegistry(ConsulRegistryOption);

impl ConsulRegistry {
    pub fn new(conf: ConsulRegistryOption) -> Self {
        Self(conf)
    }

    pub fn discover(consul: ConsulConf) -> Self {
        Self(ConsulRegistryOption::discover(consul))
    }

    pub fn register(consul: ConsulConf, service: ServiceConf) -> Self {
        Self(ConsulRegistryOption::register(consul, service))
    }
}

#[async_trait]
impl ServiceRegister for ConsulRegistry {
    type Error = consul::errors::Error;

    async fn register_service(&self, service_key: &str) -> Result<(), Self::Error> {
        let (conf, service) = match &self.0 {
            ConsulRegistryOption::Register { consul, service } => (consul.clone(), service.clone()),
            ConsulRegistryOption::Discover { .. } => {
                panic!("Cannot register service with a discover config")
            }
        };
        let consul = Consul::new(conf);
        let client = consul.make_client().await.unwrap();
        todo!()
    }
}
