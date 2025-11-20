//! Service factory wrapper for SaDi.
//!
//! This module defines [`Factory<T>`], which wraps a provider closure and manages
//! singleton or transient service lifetimes. It uses the appropriate cell/mutex for
//! thread safety, and ensures singletons are cached and reused.
//!
//! # Example
//! ```
//! use sadi::Factory;
//! use sadi::{Shared, Container};
//! use std::cell::Cell;
//!
//! struct Counter(Cell<u32>);
//! impl Counter {
//!     fn inc(&self) -> u32 {
//!         let v = self.0.get();
//!         self.0.set(v+1);
//!         v+1
//!     }
//! }
//!
//! let provider = Box::new(|_c: &Container| Shared::new(Counter(Cell::new(0))));
//!
//! let f = Factory::new(provider, false); // singleton
//! let c = Container::new();
//!
//! let a = f.provide(&c);
//! let b = f.provide(&c);
//!
//! assert!(!Shared::ptr_eq(&a, &b)); // same instances
//! ```
//!
//! For singleton, use `true` for the second argument and verify the same instance is reused.

use crate::{Container, InstanceCell, Provider, Shared};

/// Wraps a provider closure and manages singleton or transient lifetimes.
///
/// - If `singleton` is true, the first call to `provide` caches the instance and all
///   subsequent calls return the same shared pointer.
/// - If `singleton` is false, each call to `provide` returns a new instance.
///
/// Thread safety is handled by the `InstanceCell` type alias.
pub struct Factory<T: ?Sized + 'static> {
    provider: Provider<T>,
    singleton: bool,
    instance: InstanceCell<T>,
}

impl<T: ?Sized + 'static> Factory<T> {
    /// Create a new factory.
    ///
    /// - `provider`: closure that produces a `Shared<T>`
    /// - `singleton`: if true, cache and reuse the instance
    pub fn new(provider: Provider<T>, singleton: bool) -> Self {
        Self {
            provider,
            singleton,
            instance: {
                #[cfg(feature = "thread-safe")]
                {
                    std::sync::Mutex::new(None)
                }
                #[cfg(not(feature = "thread-safe"))]
                {
                    std::cell::RefCell::new(None)
                }
            },
        }
    }

    /// Provide an instance of the service.
    ///
    /// - If singleton, returns the cached instance or creates and caches it.
    /// - If transient, always calls the provider.
    pub fn provide(&self, container: &Container) -> Shared<T> {
        if self.singleton {
            // thread-safe branch
            #[cfg(feature = "thread-safe")]
            {
                let mut guard = self.instance.lock().unwrap();
                if let Some(inst) = guard.as_ref() {
                    return inst.clone();
                }
                let inst = (self.provider)(container);
                *guard = Some(inst.clone());
                inst
            }

            // non-thread-safe branch
            #[cfg(not(feature = "thread-safe"))]
            {
                let mut borrow = self.instance.borrow_mut();
                if let Some(inst) = borrow.as_ref() {
                    return inst.clone();
                }
                let inst = (self.provider)(container);
                *borrow = Some(inst.clone());
                inst
            }
        } else {
            (self.provider)(container)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Counter(Cell<u32>);
    impl Counter {
        fn inc(&self) -> u32 {
            let v = self.0.get();
            self.0.set(v + 1);
            v + 1
        }
    }

    #[test]
    fn transient_factory_gives_new_instances() {
        let provider = Box::new(|_c: &Container| Shared::new(Counter(Cell::new(0))));
        let f = Factory::new(provider, false);
        let c = Container::new();
        let a = f.provide(&c);
        let b = f.provide(&c);
        // Ensure these are different instances (pointer inequality)
        assert!(!Shared::ptr_eq(&a, &b));
    }

    #[test]
    fn singleton_factory_gives_same_instance() {
        let provider = Box::new(|_c: &Container| Shared::new(Counter(Cell::new(0))));
        let f = Factory::new(provider, true);
        let c = Container::new();
        let a = f.provide(&c);
        let b = f.provide(&c);
        // Ensure these are the same instance (pointer equality)
        assert!(Shared::ptr_eq(&a, &b));
    }
}
