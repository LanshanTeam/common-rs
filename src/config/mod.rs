use once_cell::sync::OnceCell;
use std::sync::Arc;
use tracing::info;

pub mod layer;
pub mod middleware;
pub mod service;

// Root config type
pub struct Config;

pub trait ConfigType: Clone + for<'de> serde::de::Deserialize<'de> + Default {}

impl<T> ConfigType for T where T: Clone + for<'de> serde::de::Deserialize<'de> + Default {}

/// Some useful functions for load string configuration from environment.
/// Info tips when an environment is not found and how to handle it.
pub mod env {

    use super::*;

    pub fn require(env_key: impl AsRef<str>) -> String {
        std::env::var(env_key.as_ref())
            .unwrap_or_else(|_| panic!("require an environment {}", env_key.as_ref()))
    }

    pub fn optional(env_key: impl AsRef<str>, default: impl ToString) -> String {
        std::env::var(env_key.as_ref()).unwrap_or_else(|_| {
            let ret = default.to_string();
            info!(
                "cannot found environment {}, use '{}' as default",
                env_key.as_ref(),
                ret
            );
            ret
        })
    }

    pub fn optional_some(env_key: impl AsRef<str>) -> Option<String> {
        std::env::var(env_key.as_ref()).ok().or({
            info!(
                "cannot found environment {}, use None as default",
                env_key.as_ref(),
            );
            None
        })
    }
}

pub mod register {
    use super::*;

    /// Register grabbed a closure for generating values without
    /// use static block to define a value.
    /// specify the generic type C with your own config type
    #[derive(Clone)]
    pub struct Register<C: ConfigType, T>(Arc<dyn Fn(&C) -> T + Send + Sync>);

    impl<C: ConfigType, T> Register<C, T> {
        /// Create a register that returns the same instance of a value.
        pub fn once(f: impl Fn(&C) -> T + Send + Sync + 'static) -> Self
        where
            T: Send + Sync + Clone + 'static,
        {
            let cell = OnceCell::new();
            Register(Arc::new(move |resolver| {
                cell.get_or_init(|| f(resolver)).clone()
            }))
        }

        /// Use Box::leak to create a 'static lifetime register.
        /// Used for high performance scenarios, for normal scenarios please use [Register::once]
        /// Keep in mind that the return type T will be leaked in the memory,
        /// so the size of T should be in a proper range
        pub fn once_ref(f: impl Fn(&C) -> T + Send + Sync + 'static) -> Register<C, &'static T>
        where
            T: Sync + 'static,
        {
            let cell = OnceCell::new();
            Register(Arc::new(move |resolver| {
                cell.get_or_init(|| Box::leak(Box::new(f(resolver))) as &'static T)
            }))
        }

        /// Create a register that returns a new instance of a value each time.
        pub fn factory(f: impl Fn(&C) -> T + Send + Sync + 'static) -> Self {
            Register(Arc::new(f))
        }

        /// Resolve a value
        pub fn register(&self, conf: &C) -> T {
            self.0(conf)
        }
    }
}

/// Macro used to define a config
#[macro_export]
macro_rules! define_config {
    (
        $(#[derive($($der:ident),+)])?
        $vis:vis $conf:ident $((
            $(
                $dfvis:vis $dfname:ident: $dtyp:ty,
            )*
        ))? $({
            $(
                #[$ff:ident = $ffs:literal]
                $fvis:vis $fname:ident -> $typ:ty $dft:block
            ),*
        })?
    ) => {
        #[derive(Clone, serde::Deserialize, $($($der),+)?)]
        $vis struct $conf {
            $($(
                #[serde(default)]
                $dfvis $dfname: $dtyp,
            )*)?
            $($(
                #[serde(default = $ffs)]
                $fvis $fname: $typ,
            )*)?
        }

        $(
            $(fn $ff() -> $typ $dft)?
        )*

        impl Default for $conf {
            fn default() -> Self {
                Self {
                    $($(
                        $dfname: Default::default(),
                    )*)?
                    $($(
                        $fname: $ff(),
                    )*)?
                }
            }
        }
    };
}
