use crate::injector::Injector;
use crate::instance::Instance;
use crate::runtime::Shared;
use crate::scope::Scope;

#[cfg(feature = "tracing")]
use tracing::{debug, info};

pub struct Provider<T: ?Sized + 'static> {
    pub scope: Scope,

    #[allow(clippy::type_complexity)]
    #[cfg(not(feature = "thread-safe"))]
    pub factory: Box<dyn Fn(&Injector) -> Instance<T> + 'static>,
    #[allow(clippy::type_complexity)]
    #[cfg(feature = "thread-safe")]
    pub factory: Box<dyn Fn(&Injector) -> Instance<T> + Send + Sync + 'static>,
}

#[cfg(feature = "debug")]
impl<T: ?Sized + 'static> std::fmt::Debug for Provider<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct(std::any::type_name::<Self>());

        ds.field("scope", &self.scope);

        #[cfg(feature = "thread-safe")]
        {
            ds.field(
                "factory",
                &"Box<dyn Fn(&Injector) -> Instance<T> + Send + Sync + 'static>",
            );
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            ds.field(
                "factory",
                &"Box<dyn Fn(&Injector) -> Instance<T> + 'static>",
            );
        }

        ds.finish()
    }
}

#[cfg(not(feature = "thread-safe"))]
impl<T: ?Sized + 'static> Provider<T> {
    pub fn singleton<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating singleton provider with Module scope (not thread-safe)");

        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing singleton factory for type instantiation");

                Instance::new(factory(injector))
            }),
        }
    }

    pub fn transient<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating transient provider with Transient scope (not thread-safe)");

        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing transient factory - creating new instance");

                Instance::new(factory(injector))
            }),
        }
    }

    pub fn root<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating root provider with Root scope (not thread-safe)");

        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing root factory for type instantiation");

                Instance::new(factory(injector))
            }),
        }
    }
}

#[cfg(feature = "thread-safe")]
impl<T: ?Sized + 'static> Provider<T> {
    pub fn singleton<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating singleton provider with Module scope (thread-safe)");

        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing singleton factory for type instantiation");

                Instance::new(factory(injector))
            }),
        }
    }

    pub fn transient<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating transient provider with Transient scope (thread-safe)");

        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing transient factory - creating new instance");

                Instance::new(factory(injector))
            }),
        }
    }

    pub fn root<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating root provider with Root scope (thread-safe)");

        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing root factory for type instantiation");

                Instance::new(factory(injector))
            }),
        }
    }
}
