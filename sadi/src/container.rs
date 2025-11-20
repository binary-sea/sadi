//! Dependency injection Container (thread-safe and non-thread-safe variants).
//!
//! Public API:
//! - bind_abstract / bind_abstract_singleton: register providers that return Shared-compatible values (Arc/Rc) for abstract tokens (e.g. dyn Trait)
//! - bind_concrete / bind_concrete_singleton: convenience to register providers that return concrete U for concrete token T
//! - bind_instance: register an already-created Shared<T> instance as singleton
//! - resolve / has: resolve a registered token or check presence
//!
//! This crate avoids attempting complex generic coercions inside the container; when registering
//! trait-object tokens the provider should return a Shared (Arc/Rc) or use the concrete-token
//! helpers for sized tokens. The API is intentionally explicit and simple.

use std::any::TypeId;
use std::collections::HashMap;

#[cfg(feature = "thread-safe")]
use std::sync::{Arc, RwLock};

#[cfg(not(feature = "thread-safe"))]
use std::cell::RefCell;
#[cfg(not(feature = "thread-safe"))]
use std::rc::Rc;

use crate::{Error, Factory, IntoShared, Provider, Shared, FactoriesMap};

/// The DI container.
pub struct Container {
    factories: FactoriesMap,
}

//////////////////////////////////////////////////////////////////////////////
// THREAD-SAFE implementation using Arc + RwLock
//////////////////////////////////////////////////////////////////////////////
#[cfg(feature = "thread-safe")]
impl Container {
    /// Create a new container (thread-safe).
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }

    /// Register a provider that returns a Shared<T>-compatible value (e.g. Arc<U>).
    /// This is the abstract-token variant: use when the token T may be unsized (e.g. dyn Trait).
    /// Providers must be Send + Sync in thread-safe mode.
    pub fn bind_abstract<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + Send + Sync + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            false,
        )
    }

    /// Register a singleton provider (the instance will be cached) for abstract token T.
    pub fn bind_abstract_singleton<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + Send + Sync + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            true,
        )
    }

    /// Register a provider that returns a concrete value `U` for a *concrete token* `T`.
    /// Use when the token is sized (concrete) and the provider returns a plain value `U`.
    /// The container will wrap it with `Arc::new(u)` and then call `into()` to convert to Arc<T>.
    /// The bound `Arc<U>: Into<Arc<T>>` ensures this conversion is possible (usually T == U).
    pub fn bind_concrete<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized + Send + Sync,
        U: 'static,
        F: Fn(&Container) -> U + Send + Sync + 'static,
        Arc<U>: Into<Arc<T>>,
    {
        self.bind_abstract::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Arc::new(u).into()
        })
    }

    /// Singleton variant for concrete-token providers.
    pub fn bind_concrete_singleton<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized + Send + Sync,
        U: 'static,
        F: Fn(&Container) -> U + Send + Sync + 'static,
        Arc<U>: Into<Arc<T>>,
    {
        self.bind_abstract_singleton::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Arc::new(u).into()
        })
    }

    /// Register an already-created instance as singleton. The `instance` must be IntoShared<T>.
    pub fn bind_instance<T, R>(&self, instance: R) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
    {
        let shared_instance: Shared<T> = instance.into_shared();
        self.bind_internal(
            Box::new(move |_c: &Container| shared_instance.clone()),
            true,
        )
    }

    /// Internal helper to register provider Factory<T>.
    fn bind_internal<T>(&self, provider: Provider<T>, singleton: bool) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let mut map = self.factories.write().unwrap();

        if map.contains_key(&type_id) {
            return Err(Error::factory_already_registered(type_name, "factory"));
        }
        let factory: Factory<T> = Factory::new(provider, singleton);
        let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(factory);
        map.insert(type_id, boxed);
        Ok(())
    }

    /// Resolve a registered token T, returning Shared<T>.
    pub fn resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // Push resolve guard to detect circular dependencies. The guard pops on Drop.
        let _guard = crate::ResolveGuard::push(type_name)?;

        let map = self.factories.read().unwrap();
        let boxed = match map.get(&type_id) {
            Some(b) => b,
            None => return Err(Error::service_not_registered(type_name, "factory")),
        };
        let factory = boxed
            .downcast_ref::<Factory<T>>()
            .ok_or_else(|| Error::type_mismatch(type_name))?;

        Ok(factory.provide(self))
    }

    /// Check if a provider exists for token T.
    pub fn has<T>(&self) -> bool
    where
        T: ?Sized + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let map = self.factories.read().unwrap();
        map.contains_key(&type_id)
    }
}

//////////////////////////////////////////////////////////////////////////////
// NON-THREAD-SAFE implementation using Rc + RefCell
//////////////////////////////////////////////////////////////////////////////
#[cfg(not(feature = "thread-safe"))]
impl Container {
    /// Create a new container (non-thread-safe).
    pub fn new() -> Self {
        Self {
            factories: RefCell::new(HashMap::new()),
        }
    }

    /// Register a provider that returns a Shared<T>-compatible value (e.g. Rc<U>).
    pub fn bind_abstract<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            false,
        )
    }

    /// Register a singleton provider (the instance will be cached) for abstract token T.
    pub fn bind_abstract_singleton<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            true,
        )
    }

    /// Register a provider that returns a concrete value `U` for a *concrete token* `T`.
    pub fn bind_concrete<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized,
        U: 'static,
        F: Fn(&Container) -> U + 'static,
        Rc<U>: Into<Rc<T>>,
    {
        self.bind_abstract::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Rc::new(u).into()
        })
    }

    /// Singleton variant for concrete-token providers (non-thread-safe).
    pub fn bind_concrete_singleton<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized,
        U: 'static,
        F: Fn(&Container) -> U + 'static,
        Rc<U>: Into<Rc<T>>,
    {
        self.bind_abstract_singleton::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Rc::new(u).into()
        })
    }

    /// Register an already-created instance as singleton.
    pub fn bind_instance<T, R>(&self, instance: R) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
    {
        let shared_instance: Shared<T> = instance.into_shared();
        self.bind_internal(
            Box::new(move |_c: &Container| shared_instance.clone()),
            true,
        )
    }

    /// Internal helper to register provider Factory<T>.
    fn bind_internal<T>(&self, provider: Provider<T>, singleton: bool) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let mut map = self.factories.borrow_mut();

        if map.contains_key(&type_id) {
            return Err(Error::factory_already_registered(type_name, "factory"));
        }
        let factory: Factory<T> = Factory::new(provider, singleton);
        let boxed: Box<dyn std::any::Any> = Box::new(factory);
        map.insert(type_id, boxed);
        Ok(())
    }

    /// Resolve a registered token T, returning Shared<T>.
    pub fn resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // Push resolve guard to detect circular dependencies. The guard pops on Drop.
        let _guard = crate::ResolveGuard::push(type_name)?;

        let map = self.factories.borrow();
        let boxed = match map.get(&type_id) {
            Some(b) => b,
            None => return Err(Error::service_not_registered(type_name, "factory")),
        };
        let factory = boxed
            .downcast_ref::<Factory<T>>()
            .ok_or_else(|| Error::type_mismatch(type_name))?;

        Ok(factory.provide(self))
    }

    /// Check if a provider exists for token T.
    pub fn has<T>(&self) -> bool
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let map = self.factories.borrow();
        map.contains_key(&type_id)
    }
}