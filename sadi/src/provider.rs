//! Service provider definitions for dependency injection.
//!
//! This module defines the [`Provider`] struct, which encapsulates a factory function
//! and scope information for creating and managing service instances.
//!
//! # Overview
//!
//! Providers are the building blocks of dependency injection. They define:
//! - How to create an instance of a service (via a factory function)
//! - When and how often to create new instances (via the scope)
//!
//! # Scopes
//!
//! - **Singleton (Module)**: A single instance is created per module and reused
//! - **Transient**: A new instance is created every time the service is requested
//!
//! # Feature Flags
//!
//! The provider implementation varies based on the `thread-safe` feature:
//! - **With `thread-safe`**: Factory functions and types must be `Send + Sync`
//! - **Without `thread-safe`**: Single-threaded factories and types are allowed
//!
//! # Examples
//!
//! ```
//! use sadi::provider::Provider;
//! use sadi::injector::Injector;
//!
//! // Create a singleton provider
//! let provider = Provider::singleton(|injector| {
//!     42u32
//! });
//!
//! // Create a transient provider
//! let provider = Provider::transient(|injector| {
//!     "Hello, World!".to_string()
//! });
//! ```

use std::any::Any;

use crate::injector::Injector;
use crate::runtime::Shared;
use crate::scope::Scope;

#[cfg(feature = "tracing")]
use tracing::{debug, info};

/// A factory function wrapper that manages service creation and lifecycle scope.
///
/// A provider encapsulates both a factory function (that creates instances) and
/// a scope (that determines the instance lifecycle). The factory receives an injector
/// to allow dependency resolution within the factory function.
///
/// # Fields
///
/// * `scope` - The lifecycle scope (`Module` for singleton, `Transient` for new instances)
/// * `factory` - A closure that creates instances of the service
///
/// # Type Parameters
///
/// The factory is a generic function that can create any type `T` that implements `Any`.
/// The exact signature depends on the `thread-safe` feature flag.
///
/// # Examples
///
/// ```
/// use sadi::provider::Provider;
///
/// // Singleton provider for a simple type
/// let provider = Provider::singleton(|_injector| {
///     vec![1, 2, 3]
/// });
///
/// // Transient provider with dependency injection
/// let provider = Provider::transient(|injector| {
///     String::from("Created at request time")
/// });
/// ```
pub struct Provider {
    pub scope: Scope,

    #[cfg(not(feature = "thread-safe"))]
    pub factory: Box<dyn Fn(Shared<Injector>) -> Shared<dyn Any> + 'static>,
    #[cfg(feature = "thread-safe")]
    pub factory:
        Box<dyn Fn(Shared<Injector>) -> Shared<dyn Any + Send + Sync> + Send + Sync + 'static>,
}

#[cfg(feature = "thread-safe")]
impl Provider {
    /// Creates a singleton provider with module-level scope.
    ///
    /// A singleton provider creates a single instance per module that is reused
    /// for all consumers within that module. The instance is created once when
    /// first requested and then cached.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The type of the service being provided. Must implement `Any + Send + Sync + 'static`.
    /// - `F`: The factory function type. Must be `Send + Sync + 'static`.
    ///
    /// # Parameters
    ///
    /// - `factory`: A closure that takes an injector and returns an instance of type `T`
    ///
    /// # Returns
    ///
    /// A new `Provider` configured as a module-level singleton.
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::provider::Provider;
    ///
    /// let provider = Provider::singleton(|_injector| {
    ///     42u32
    /// });
    ///
    /// let provider = Provider::singleton(|_injector| {
    ///     "singleton service".to_string()
    /// });
    /// ```
    pub fn singleton<T, F>(factory: F) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: Fn(Shared<Injector>) -> T + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating singleton provider with Module scope (thread-safe)");

        Self {
            scope: Scope::Module,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing singleton factory for type instantiation");

                Shared::new(factory(injector)) as Shared<dyn Any + Send + Sync>
            }),
        }
    }

    /// Creates a transient provider that creates new instances on each request.
    ///
    /// A transient provider creates a fresh instance every time the service is
    /// requested. No caching occurs, making it suitable for stateful services
    /// that should not be shared between consumers.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The type of the service being provided. Must implement `Any + Send + Sync + 'static`.
    /// - `F`: The factory function type. Must be `Send + Sync + 'static`.
    ///
    /// # Parameters
    ///
    /// - `factory`: A closure that takes an injector and returns a new instance of type `T`
    ///
    /// # Returns
    ///
    /// A new `Provider` configured with transient scope.
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::provider::Provider;
    ///
    /// let provider = Provider::transient(|_injector| {
    ///     vec![] as Vec<String>
    /// });
    ///
    /// let provider = Provider::transient(|_injector| {
    ///     std::collections::HashMap::new()
    /// });
    /// ```
    pub fn transient<T, F>(factory: F) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: Fn(Shared<Injector>) -> T + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating transient provider with Transient scope (thread-safe)");

        Self {
            scope: Scope::Transient,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing transient factory - creating new instance");

                Shared::new(factory(injector)) as Shared<dyn Any + Send + Sync>
            }),
        }
    }
}

#[cfg(not(feature = "thread-safe"))]
impl Provider {
    /// Creates a singleton provider with module-level scope.
    ///
    /// A singleton provider creates a single instance per module that is reused
    /// for all consumers within that module. The instance is created once when
    /// first requested and then cached.
    ///
    /// This is the single-threaded variant without `Send + Sync` requirements.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The type of the service being provided. Must implement `Any + 'static`.
    /// - `F`: The factory function type. Must be `'static`.
    ///
    /// # Parameters
    ///
    /// - `factory`: A closure that takes an injector and returns an instance of type `T`
    ///
    /// # Returns
    ///
    /// A new `Provider` configured as a module-level singleton.
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::provider::Provider;
    ///
    /// let provider = Provider::singleton(|_injector| {
    ///     42u32
    /// });
    ///
    /// let provider = Provider::singleton(|_injector| {
    ///     "singleton service".to_string()
    /// });
    /// ```
    pub fn singleton<T, F>(factory: F) -> Self
    where
        T: Any + 'static,
        F: Fn(Shared<Injector>) -> T + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating singleton provider with Module scope (single-threaded)");

        Self {
            scope: Scope::Module,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing singleton factory for type instantiation");

                Shared::new(factory(injector)) as Shared<dyn Any>
            }),
        }
    }

    /// Creates a transient provider that creates new instances on each request.
    ///
    /// A transient provider creates a fresh instance every time the service is
    /// requested. No caching occurs, making it suitable for stateful services
    /// that should not be shared between consumers.
    ///
    /// This is the single-threaded variant without `Send + Sync` requirements.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The type of the service being provided. Must implement `Any + 'static`.
    /// - `F`: The factory function type. Must be `'static`.
    ///
    /// # Parameters
    ///
    /// - `factory`: A closure that takes an injector and returns a new instance of type `T`
    ///
    /// # Returns
    ///
    /// A new `Provider` configured with transient scope.
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::provider::Provider;
    ///
    /// let provider = Provider::transient(|_injector| {
    ///     vec![] as Vec<String>
    /// });
    ///
    /// let provider = Provider::transient(|_injector| {
    ///     std::collections::HashMap::<String, String>::new()
    /// });
    /// ```
    pub fn transient<T, F>(factory: F) -> Self
    where
        T: Any + 'static,
        F: Fn(Shared<Injector>) -> T + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating transient provider with Transient scope (single-threaded)");

        Self {
            scope: Scope::Transient,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing transient factory - creating new instance");

                Shared::new(factory(injector)) as Shared<dyn Any>
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleton_creates_module_scope() {
        let provider = Provider::singleton(|_| 42u32);
        assert!(matches!(provider.scope, Scope::Module), "Singleton should have Module scope");
    }

    #[test]
    fn test_transient_creates_transient_scope() {
        let provider = Provider::transient(|_| "hello".to_string());
        assert!(
            matches!(provider.scope, Scope::Transient),
            "Transient should have Transient scope"
        );
    }

    #[test]
    fn test_singleton_factory_returns_valid_value() {
        let provider = Provider::singleton(|_| 100u32);
        let injector = Shared::new(Injector::root());
        let value = (provider.factory)(injector);

        let value = value.downcast_ref::<u32>().unwrap();
        assert_eq!(*value, 100, "Singleton factory should return the correct value");
    }

    #[test]
    fn test_transient_factory_creates_new_instances() {
        let provider = Provider::transient(|_| "transient".to_string());
        let injector = Shared::new(Injector::root());

        let first = (provider.factory)(injector.clone());
        let second = (provider.factory)(injector);

        assert!(
            !Shared::ptr_eq(&first, &second),
            "Transient factory should create different instances"
        );
    }

    #[test]
    fn test_singleton_factory_with_different_types() {
        let int_provider = Provider::singleton(|_| 42u32);
        let string_provider = Provider::singleton(|_| String::from("test"));
        let vec_provider = Provider::singleton(|_| vec![1, 2, 3]);

        let injector = Shared::new(Injector::root());

        let int_val = (int_provider.factory)(injector.clone());
        assert_eq!(*int_val.downcast_ref::<u32>().unwrap(), 42);

        let string_val = (string_provider.factory)(injector.clone());
        assert_eq!(*string_val.downcast_ref::<String>().unwrap(), "test");

        let vec_val = (vec_provider.factory)(injector);
        assert_eq!(*vec_val.downcast_ref::<Vec<i32>>().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_transient_factory_with_different_types() {
        let string_provider = Provider::transient(|_| String::from("transient"));
        let vec_provider = Provider::transient(|_| vec![1, 2, 3, 4, 5]);

        let injector = Shared::new(Injector::root());

        let first_string = (string_provider.factory)(injector.clone());
        let second_string = (string_provider.factory)(injector.clone());

        assert!(
            !Shared::ptr_eq(&first_string, &second_string),
            "Transient strings should be different instances"
        );

        let first_vec = (vec_provider.factory)(injector.clone());
        let second_vec = (vec_provider.factory)(injector);

        assert!(
            !Shared::ptr_eq(&first_vec, &second_vec),
            "Transient vectors should be different instances"
        );
    }

    #[test]
    fn test_multiple_providers() {
        let providers = vec![
            Provider::singleton(|_| 1u32),
            Provider::singleton(|_| 2u32),
            Provider::transient(|_| String::from("a")),
            Provider::transient(|_| String::from("b")),
        ];

        assert_eq!(providers.len(), 4);

        let singleton_count = providers.iter().filter(|p| p.scope.is_singleton()).count();
        assert_eq!(singleton_count, 2, "Should have 2 singleton providers");

        let transient_count = providers.iter().filter(|p| !p.scope.is_singleton()).count();
        assert_eq!(transient_count, 2, "Should have 2 transient providers");
    }

    #[test]
    fn test_provider_with_complex_type() {
        struct Service {
            id: usize,
            name: String,
        }

        let provider = Provider::singleton(|_| Service {
            id: 123,
            name: "test".to_string(),
        });

        let injector = Shared::new(Injector::root());
        let value = (provider.factory)(injector);
        let service = value.downcast_ref::<Service>().unwrap();

        assert_eq!(service.id, 123);
        assert_eq!(service.name, "test");
    }
}

