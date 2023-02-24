use crate::config::register::Register;
use crate::config::ConfigType;
use std::fmt::{Display, Formatter};

/// The target service type to be resolved by the resolver.
pub enum Target {
    REST,    // restful service
    GRPC,    // grpc service
    GRAPHQL, // graphql service
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::REST => write!(f, "rest"),
            Target::GRPC => write!(f, "grpc"),
            Target::GRAPHQL => write!(f, "graphql"),
        }
    }
}

/// Basic abstraction of Resolver
pub trait Resolver {
    /// The target to be resolved by the resolver.
    const TARGET: Target;
    /// The domain of target.
    const DOMAIN: &'static str;
    /// The config type hold by the resolver.
    type Config: ConfigType;

    /// Return the reference of the config hold by the resolver
    fn conf(&self) -> &Self::Config;

    /// config hint used to generate configuration structure
    fn conf_hint() -> Self::Config {
        Self::Config::default()
    }

    /// A service key concat the system domain and exposed api type.
    /// It needs to be unique in the whole system, so it could be used
    /// in service register/discover
    fn service_key() -> String {
        format!("{}-{}", Self::DOMAIN, Self::TARGET)
    }

    /// Resolve a register.
    fn resolve<T>(&self, register: &Register<Self::Config, T>) -> T {
        register.register(self.conf())
    }
}

#[cfg(test)]
mod test {
    use crate::config::middleware::MiddlewareConfig;
    use crate::config::register::Register;
    use crate::config::service::ServiceConfig;
    use crate::config::Config;
    use crate::infra::{Resolver, Target};
    use serde::{Deserialize, Serialize};

    type MyRegister<T> = Register<MyConfig, T>;

    struct MyResolver {
        conf: MyConfig,
        redis: MyRegister<redis::Client>,
    }

    #[derive(Debug, Default, Deserialize, Serialize, Clone)]
    struct MyConfig {
        service_conf: <Config as ServiceConfig>::GrpcService,
        redis_conf: <Config as MiddlewareConfig>::Redis,
    }

    impl Resolver for MyResolver {
        const TARGET: Target = Target::GRPC;
        const DOMAIN: &'static str = "sys";
        type Config = MyConfig;

        fn conf(&self) -> &Self::Config {
            &self.conf
        }
    }

    impl MyResolver {
        fn new(conf: MyConfig) -> Self {
            MyResolver {
                conf,
                redis: MyRegister::once(|conf| {
                    redis::Client::open(conf.redis_conf.dsn.as_str()).unwrap()
                }),
            }
        }

        fn redis(&self) -> redis::Client {
            self.resolve(&self.redis)
        }
    }

    #[tokio::test]
    async fn test() {
        let resolver = MyResolver::new(MyConfig::default());
        let client = resolver.redis();
        println!("{:?}", client);
    }
}
