use super::*;

pub trait MiddlewareConfig {
    type Etcd: ConfigType;
    type Consul: ConfigType;
    type Apollo: ConfigType;
    type Nacos: ConfigType;
    type Redis: ConfigType;
    type RabbitMQ: ConfigType;
}

impl MiddlewareConfig for Config {
    type Etcd = crate::middleware::etcd::EtcdConf;
    type Consul = crate::middleware::consul::ConsulConf;
    type Apollo = crate::middleware::apollo::ApolloConf;
    type Nacos = crate::middleware::nacos::NacosConf;
    type Redis = crate::middleware::redis::RedisConf;
    type RabbitMQ = crate::middleware::rabbitmq::RabbitMQConf;
}
