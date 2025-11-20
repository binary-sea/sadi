//! # Dependency Injection Container
//!
//! This module provides a minimal, flexible, feature-flag-driven
//! Dependency Injection (DI) and Inversion of Control (IoC) system for Rust.
//!
//! It is designed to be:
//!
//! - **Ergonomic** — Zero boilerplate usage.
//! - **Flexible** — Works with traits, structs, lambdas, factories, singletons.
//! - **Safe** — Detects circular dependencies at runtime.
//! - **Configurable** — Can run in:
//!   - **Thread-safe mode (`feature = "thread-safe"`)** using `Arc` + `RwLock`
//!   - **Non-thread-safe mode** using `Rc` + `RefCell`
//!
//! ## Core Concepts
//!
//! - **Binding**  
//!   You register a provider function (factory) that knows how to build a type.
//!
//! - **Resolving**  
//!   You request an instance of a type, and the container builds (or retrieves)
//!   it for you.
//!
//! - **Singletons**  
//!   A singleton is created once and cached.
//!
//! - **Transient bindings**  
//!   A new instance is created every time.
//!
//! - **Shared pointers**  
//!   Depending on the feature set, the container transparently uses
//!   `Arc<T>` or `Rc<T>`.
//!
//! ## When to Use This Library?
//!
//! - Building servers using **Axum**, **Actix**, **Rocket**, etc.
//! - Creating modular business layers following **DDD**.
//! - Managing plugin-based architectures.
//! - Replacing complex dependency graphs with clean, minimal DI code.
//!
//! ## Basic Example
//!
//! ```rust
//! use sadi::Container;
//!
//! struct Service {
//!     pub value: i32,
//! }
//!
//! let c = Container::new();
//!
//! c.bind_concrete::<Service, Service, _>(|_| Service { value: 10 }).unwrap();
//!
//! let s = c.resolve::<Service>().unwrap();
//! assert_eq!(s.value, 10);
//! ```
//!
//! ## Trait Binding Example
//!
//! ```rust
//! use sadi::Container;
//! use std::sync::Arc;
//!
//! trait Repo: Send + Sync {
//!     fn n(&self) -> i32;
//! }
//!
//! struct RepoImpl;
//! impl Repo for RepoImpl {
//!     fn n(&self) -> i32 { 42 }
//! }
//!
//! let c = Container::new();
//!
//! c.bind_abstract::<dyn Repo, _, _>(|_| Arc::new(RepoImpl) as Arc<dyn Repo>).unwrap();
//!
//! let repo = c.resolve::<dyn Repo>().unwrap();
//! assert_eq!(repo.n(), 42);
//! ```
//!
//! ## Singleton Example
//!
//! ```rust
//! use sadi::Container;
//! use std::sync::Mutex;
//!
//! struct Counter(Mutex<i32>);
//!
//! let c = Container::new();
//!
//! c.bind_concrete_singleton::<Counter, Counter, _>(|_| Counter(Mutex::new(0))).unwrap();
//!
//! let a = c.resolve::<Counter>().unwrap();
//! *a.0.lock().unwrap() = 99;
//!
//! let b = c.resolve::<Counter>().unwrap();
//! assert_eq!(*b.0.lock().unwrap(), 99);
//! ```
//!
//! ## Complex Example: Service Graph
//!
//! ```rust
//! use sadi::Container;
//! use std::sync::Arc;
//!
//! trait Logger: Send + Sync {
//!     fn log(&self, msg: &str);
//! }
//!
//! struct ConsoleLogger;
//! impl Logger for ConsoleLogger {
//!     fn log(&self, msg: &str) { println!("LOG: {}", msg); }
//! }
//!
//! struct UserRepository {
//!     logger: std::sync::Arc<dyn Logger>,
//! }
//! impl UserRepository {
//!     fn new(logger: std::sync::Arc<dyn Logger>) -> Self { Self { logger } }
//! }
//!
//! struct UserService {
//!     repo: std::sync::Arc<UserRepository>,
//! }
//! impl UserService {
//!     fn new(repo: std::sync::Arc<UserRepository>) -> Self { Self { repo } }
//! }
//!
//! let c = Container::new();
//!
//! c.bind_abstract::<dyn Logger, _, _>(|_| Arc::new(ConsoleLogger) as Arc<dyn Logger>).unwrap();
//!
//! c.bind_concrete::<UserRepository, UserRepository, _>(|c| {
//!     let logger = c.resolve::<dyn Logger>().unwrap();
//!     UserRepository::new(logger)
//! }).unwrap();
//!
//! c.bind_concrete::<UserService, UserService, _>(|c| {
//!     let repo = c.resolve::<UserRepository>().unwrap();
//!     UserService::new(repo)
//! }).unwrap();
//!
//! let svc = c.resolve::<UserService>().unwrap();
//! svc.repo.logger.log("User loaded");
//! ```
//!
//! ## Circular Dependency Detection
//!
//! ```rust,ignore
//! use sadi::Container;
//! use std::sync::Arc;
//!
//! #[derive(Debug)]
//! struct A;
//!
//! #[derive(Debug)]
//! struct B;
//!
//! let c = Container::new();
//!
//! c.bind_concrete::<A, A, _>(|c| {
//!     let _ = c.resolve::<B>().unwrap();
//!     A
//! }).unwrap();
//!
//! c.bind_concrete::<B, B, _>(|c| {
//!     let _ = c.resolve::<A>().unwrap();
//!     B
//! }).unwrap();
//!
//! let err = c.resolve::<A>();
//! assert_eq!(err.is_ok(), false);
//! ```

use std::any::TypeId;
use std::collections::HashMap;

#[cfg(feature = "thread-safe")]
use std::sync::{Arc, RwLock};

#[cfg(not(feature = "thread-safe"))]
use std::cell::RefCell;
#[cfg(not(feature = "thread-safe"))]
use std::rc::Rc;

use crate::{Error, FactoriesMap, Factory, IntoShared, Provider, Shared};

/// The IoC/DI container.
///
/// Stores provider functions (`Factory<T>`) indexed by `TypeId`,
/// and builds values on demand using dependency resolution.
pub struct Container {
    factories: FactoriesMap,
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

//
// ──────────────────────────────────────────────────────────────────────────────
//   THREAD SAFE IMPLEMENTATION (feature = "thread-safe")
// ──────────────────────────────────────────────────────────────────────────────
//

#[cfg(feature = "thread-safe")]
impl Container {
    /// Creates a new thread-safe container backed by `RwLock<HashMap>`.
    ///
    /// # Example
    /// ```
    /// use sadi::Container;
    ///
    /// let c = Container::new();
    /// assert!(!c.has::<i32>());
    /// ```
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }

    /// Registers an **abstract binding** for trait objects or unsized types.
    ///
    /// Instances are **not cached** (transient).
    pub fn bind_abstract<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + Send + Sync + 'static,
    {
        self.bind_internal(Box::new(move |c| provider(c).into_shared()), false)
    }

    /// Registers a **singleton** abstract binding.
    pub fn bind_abstract_singleton<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + Send + Sync + 'static,
    {
        self.bind_internal(Box::new(move |c| provider(c).into_shared()), true)
    }

    /// Registers a concrete implementation automatically wrapped in `Arc`.
    pub fn bind_concrete<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Send + Sync,
        U: 'static,
        F: Fn(&Container) -> U + Send + Sync + 'static,
        Arc<U>: Into<Arc<T>>,
    {
        self.bind_abstract::<T, _, _>(move |c| Arc::new(provider(c)).into())
    }

    /// Singleton version of [`bind_concrete`].
    pub fn bind_concrete_singleton<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Send + Sync,
        U: 'static,
        F: Fn(&Container) -> U + Send + Sync + 'static,
        Arc<U>: Into<Arc<T>>,
    {
        self.bind_abstract_singleton::<T, _, _>(move |c| Arc::new(provider(c)).into())
    }

    /// Registers an already created instance as a singleton.
    pub fn bind_instance<T, R>(&self, instance: R) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
        R: IntoShared<T> + 'static,
    {
        let shared = instance.into_shared();

        self.bind_internal(Box::new(move |_| shared.clone()), true)
    }

    /// Internal binding logic shared by all binding methods.
    fn bind_internal<T>(&self, provider: Provider<T>, singleton: bool) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        let name = std::any::type_name::<T>();

        let mut map = self.factories.write().unwrap();

        if map.contains_key(&id) {
            return Err(Error::factory_already_registered(name, "factory"));
        }

        let factory = Factory::new(provider, singleton);

        map.insert(id, Box::new(factory));

        Ok(())
    }

    /// Resolves a previously registered binding.
    ///
    /// Performs:
    /// - Type lookup  
    /// - Circular dependency detection  
    /// - Singleton caching  
    /// - Provider invocation  
    pub fn resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        let name = std::any::type_name::<T>();

        let _guard = crate::ResolveGuard::push(name)?;

        let map = self.factories.read().unwrap();
        let boxed = map
            .get(&id)
            .ok_or_else(|| Error::service_not_registered(name, "factory"))?;

        let factory = boxed
            .downcast_ref::<Factory<T>>()
            .ok_or_else(|| Error::type_mismatch(name))?;

        Ok(factory.provide(self))
    }

    /// Returns `true` if a type has been registered.
    pub fn has<T>(&self) -> bool
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        self.factories.read().unwrap().contains_key(&id)
    }
}

//
// ──────────────────────────────────────────────────────────────────────────────
//   NON THREAD SAFE IMPLEMENTATION (default)
// ──────────────────────────────────────────────────────────────────────────────
//

#[cfg(not(feature = "thread-safe"))]
impl Container {
    /// Creates a new non-thread-safe container backed by `RefCell<HashMap>`.
    ///
    /// # Example
    /// ```
    /// let c = Container::new();
    /// assert!(!c.has::<i32>());
    /// ```
    pub fn new() -> Self {
        Self {
            factories: RefCell::new(HashMap::new()),
        }
    }

    /// Registers an **abstract binding** for trait objects or unsized types.
    ///
    /// Instances are **not cached** (transient).
    pub fn bind_abstract<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + 'static,
    {
        self.bind_internal(Box::new(move |c| provider(c).into_shared()), false)
    }

    /// Registers a **singleton** abstract binding.
    pub fn bind_abstract_singleton<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + 'static,
    {
        self.bind_internal(Box::new(move |c| provider(c).into_shared()), true)
    }

    /// Registers a concrete implementation automatically wrapped in `Rc`.
    pub fn bind_concrete<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static,
        U: 'static,
        F: Fn(&Container) -> U + 'static,
        Rc<U>: Into<Rc<T>>,
    {
        self.bind_abstract::<T, _, _>(move |c| Rc::new(provider(c)).into())
    }

    /// Singleton version of [`bind_concrete`].
    pub fn bind_concrete_singleton<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static,
        U: 'static,
        F: Fn(&Container) -> U + 'static,
        Rc<U>: Into<Rc<T>>,
    {
        self.bind_abstract_singleton::<T, _, _>(move |c| Rc::new(provider(c)).into())
    }

    /// Registers an already created instance as a singleton.
    pub fn bind_instance<T, R>(&self, instance: R) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
    {
        let shared = instance.into_shared();

        self.bind_internal(Box::new(move |_| shared.clone()), true)
    }

    /// Internal binding logic shared by all binding methods.
    fn bind_internal<T>(&self, provider: Provider<T>, singleton: bool) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let id = TypeId::of::<T>();
        let name = std::any::type_name::<T>();

        let mut map = self.factories.borrow_mut();

        if map.contains_key(&id) {
            return Err(Error::factory_already_registered(name, "factory"));
        }

        map.insert(id, Box::new(Factory::new(provider, singleton)));

        Ok(())
    }

    /// Resolves a previously registered binding.
    ///
    /// Performs:
    /// - Type lookup  
    /// - Circular dependency detection  
    /// - Singleton caching  
    /// - Provider invocation  
    pub fn resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        let id = TypeId::of::<T>();
        let name = std::any::type_name::<T>();

        let _guard = crate::ResolveGuard::push(name)?;

        let map = self.factories.borrow();
        let boxed = map
            .get(&id)
            .ok_or_else(|| Error::service_not_registered(name, "factory"))?;

        let factory = boxed
            .downcast_ref::<Factory<T>>()
            .ok_or_else(|| Error::type_mismatch(name))?;

        Ok(factory.provide(self))
    }

    /// Returns whether type `T` has a registered factory.
    pub fn has<T>(&self) -> bool
    where
        T: ?Sized + 'static,
    {
        let id = TypeId::of::<T>();
        self.factories.borrow().contains_key(&id)
    }
}

//
// ──────────────────────────────────────────────────────────────────────────────
//   TESTS
// ──────────────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;

    struct S(pub i32);

    #[test]
    fn bind_and_resolve_concrete() {
        let c = Container::new();
        c.bind_concrete::<S, S, _>(|_c| S(7)).unwrap();
        let s = c.resolve::<S>().unwrap();
        assert_eq!((*s).0, 7);
    }

    #[test]
    fn bind_instance_and_singleton_behavior() {
        let c = Container::new();
        let instance = Shared::new(S(5));
        c.bind_instance::<S, _>(instance).unwrap();
        assert!(c.has::<S>());

        let a = c.resolve::<S>().unwrap();
        let b = c.resolve::<S>().unwrap();
        let pa = (&*a) as *const S;
        let pb = (&*b) as *const S;
        assert_eq!(pa, pb);
    }

    #[test]
    fn resolve_guard_detects_cycle() {
        let _g1 = crate::ResolveGuard::push("A").unwrap();
        let _g2 = crate::ResolveGuard::push("B").unwrap();
        let err = crate::ResolveGuard::push("A").unwrap_err();
        assert_eq!(err.kind, crate::ErrorKind::CircularDependency);
    }
}
