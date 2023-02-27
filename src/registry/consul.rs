use crate::config::service::ServiceConf;
use crate::middleware::consul::{Consul, ConsulConf};
use crate::middleware::Middleware;
use crate::registry::{ConsulRegistryOption, ServiceRegister};
use async_trait::async_trait;
use consul::agent::{Agent, RegisterAgentService};

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
        let (
            conf,
            service,
            replace_existing_checks,
            enable_tag_override,
            tags,
            meta,
            check,
            weights,
        ) = match &self.0 {
            ConsulRegistryOption::Register {
                consul,
                service,
                replace_existing_checks,
                enable_tag_override,
                tags,
                meta,
                check,
                weights,
            } => (
                consul.clone(),
                service,
                *replace_existing_checks,
                *enable_tag_override,
                tags.clone(),
                meta.clone(),
                check.as_deref().map(ToOwned::to_owned),
                weights.clone(),
            ),
            ConsulRegistryOption::Discover { .. } => {
                panic!("Cannot register service with a discover config")
            }
        };
        let consul = Consul::new(conf);
        let client = consul.make_client().await.unwrap();
        let discover_url =
            url::Url::parse(&service.discover_addr).expect("Not a valid discover addr");
        let address = discover_url.host_str().expect("Not a valid discover addr");
        let port = discover_url
            .port_or_known_default()
            .expect("Not a valid discover addr");
        client
            .register_service(
                &RegisterAgentService {
                    Name: service_key.to_string(),
                    ID: format!("{}:{}", service_key, service.name),
                    Address: address.to_string(),
                    Port: port,
                    EnableTagOverride: enable_tag_override,
                    Tags: tags,
                    Meta: meta,
                    Check: check,
                    Weights: weights,
                    ..Default::default()
                },
                replace_existing_checks,
            )
            .await?;
        Ok(())
    }
}

// TODO consul ServiceDiscover
// optional, we can use consul dns resolver to discover service