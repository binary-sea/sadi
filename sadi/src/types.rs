//! Internal type aliases and helpers for SaDi's container implementation.
//!
//! This module provides thread-safe and non-thread-safe variants of core types
//! (such as provider functions, instance cells, and factory maps) depending on
//! the `thread-safe` feature flag. These types are used internally by the DI
//! container to manage service registration and resolution.
use crate::Shared;
use std::{any::TypeId, collections::HashMap};

use crate::Container;

/// Type alias for a service provider (factory function).
///
/// - In thread-safe mode, the provider closure must be `Send + Sync`.
/// - In single-threaded mode, only `'static` is required.
///
/// The provider receives a reference to the container and returns a shared instance.
#[cfg(feature = "thread-safe")]
pub type Provider<T> = Box<dyn Fn(&Container) -> Shared<T> + Send + Sync + 'static>;
#[cfg(not(feature = "thread-safe"))]
pub type Provider<T> = Box<dyn Fn(&Container) -> Shared<T> + 'static>;

/// Type alias for a cell holding a singleton/shared instance.
///
/// - In thread-safe mode, uses `Mutex` for safe concurrent access.
/// - In single-threaded mode, uses `RefCell` for fast interior mutability.
#[cfg(feature = "thread-safe")]
pub type InstanceCell<T> = std::sync::Mutex<Option<Shared<T>>>;
#[cfg(not(feature = "thread-safe"))]
pub type InstanceCell<T> = std::cell::RefCell<Option<Shared<T>>>;

/// Type alias for the map storing all registered service factories.
///
/// - In thread-safe mode, uses `RwLock` for concurrent reads/writes and requires
///   all stored factories to be `Send + Sync`.
/// - In single-threaded mode, uses `RefCell` for fast interior mutability.
#[cfg(feature = "thread-safe")]
pub type FactoriesMap = std::sync::RwLock<HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>>;
#[cfg(not(feature = "thread-safe"))]
pub type FactoriesMap = std::cell::RefCell<HashMap<TypeId, Box<dyn std::any::Any>>>;
