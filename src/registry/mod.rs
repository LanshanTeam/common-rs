pub mod etcd;

use crate::config::service::ServiceConf;
use crate::middleware::consul::ConsulConf;
use crate::middleware::etcd::EtcdConf;
use async_trait::async_trait;
pub use etcd::*;
use std::hash::Hash;
use tokio::sync::mpsc::Sender;
use tonic::transport::Endpoint;
use tower::discover::Change;

/// `service_key` must be unique crossing all service
/// see [`Resolver::service_key`]
///
/// [`Resolver`]: crate::infra::Resolver
#[async_trait]
pub trait ServiceRegister<K>
where
    K: Hash + Eq + Send + Clone + 'static,
{
    type Error;

    async fn register_service(&self, service_key: &str) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait ServiceDiscover<K>
where
    K: Hash + Eq + Send + Clone + 'static,
{
    type Error;

    async fn discover_to_channel(
        &self,
        service_key: &str,
        tx: Sender<Change<K, Endpoint>>,
    ) -> Result<(), Self::Error>;
}

// The combination of discovery and registration services.
// It is not suitable for use in a custom configuration, so
// it does not derive serde traits.
#[derive(Clone, Debug)]
pub enum EtcdRegistryOption {
    Register {
        etcd: EtcdConf,
        service: ServiceConf,
        grant_ttl: i64,
        keep_alive_interval: u64,
    },
    Discover {
        etcd: EtcdConf,
    },
}

impl EtcdRegistryOption {
    pub fn discover(etcd: EtcdConf) -> Self {
        Self::Discover { etcd }
    }

    pub fn register(etcd: EtcdConf, service: ServiceConf) -> Self {
        Self::Register {
            etcd,
            service,
            grant_ttl: 61,
            keep_alive_interval: 20,
        }
    }

    pub fn grant_ttl(mut self, ttl: i64) -> Self {
        if let EtcdRegistryOption::Register { grant_ttl, .. } = &mut self {
            *grant_ttl = ttl;
        }
        self
    }

    pub fn keep_alive_interval(mut self, kai: u64) -> Self {
        if let EtcdRegistryOption::Register {
            keep_alive_interval,
            ..
        } = &mut self
        {
            *keep_alive_interval = kai;
        }
        self
    }
}

impl Default for EtcdRegistryOption {
    fn default() -> Self {
        Self::Discover {
            etcd: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConsulRegistryOption {
    Register {
        consul: ConsulConf,
        service: ServiceConf,
    },
    Discover {
        consul: ConsulConf,
    },
}

impl Default for ConsulRegistryOption {
    fn default() -> Self {
        Self::Discover {
            consul: Default::default(),
        }
    }
}
