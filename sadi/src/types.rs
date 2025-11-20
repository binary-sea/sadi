use std::{any::TypeId, collections::HashMap};
use crate::Shared;

use crate::Container;

#[cfg(feature = "thread-safe")]
pub type Provider<T> = Box<dyn Fn(&Container) -> Shared<T> + Send + Sync + 'static>;
#[cfg(not(feature = "thread-safe"))]
pub type Provider<T> = Box<dyn Fn(&Container) -> Shared<T> + 'static>;

#[cfg(feature = "thread-safe")]
pub type InstanceCell<T> = std::sync::Mutex<Option<Shared<T>>>;
#[cfg(not(feature = "thread-safe"))]
pub type InstanceCell<T> = std::cell::RefCell<Option<Shared<T>>>;

#[cfg(feature = "thread-safe")]
pub type FactoriesMap = std::sync::RwLock<HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>>;
#[cfg(not(feature = "thread-safe"))]
pub type FactoriesMap = std::cell::RefCell<HashMap<TypeId, Box<dyn std::any::Any>>>;
