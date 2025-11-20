//! Shared pointer abstraction and conversion helpers for SaDi.
//!
//! This module provides a unified `Shared<T>` type alias that is `Arc<T>` in thread-safe mode
//! and `Rc<T>` otherwise. It also defines the [`IntoShared`] trait, which allows provider
//! return types to be converted into `Shared<T>`, supporting both concrete and trait-object
//! use cases.
//!
//! # Examples
//!
//! ## Thread-safe mode (`feature = "thread-safe"`)
//! ```rust
//! use sadi::Shared;
//! use std::sync::Arc;
//!
//! let x: Arc<u32> = Arc::new(42);
//! let y: Shared<u32> = x.clone();
//! assert_eq!(*y, 42);
//! ```
//!
//! ## Non-thread-safe mode (default)
//! ```rust
//! use sadi::Shared;
//! use std::sync::Arc;
//!
//! let x: Arc<u32> = Arc::new(42);
//! let y: Shared<u32> = x.clone();
//! assert_eq!(*y, 42);
//! ```
//!
//! ## Using IntoShared for trait objects
//! ```rust
//! use sadi::{Shared, IntoShared};
//! use std::sync::Arc;
//!
//! trait Foo { fn foo(&self) -> i32; }
//! struct Bar;
//! impl Foo for Bar { fn foo(&self) -> i32 { 7 } }
//!
//! let arc: Arc<dyn Foo> = Arc::new(Bar);
//! let shared: Shared<dyn Foo> = arc.into_shared();
//!
//! assert_eq!(shared.foo(), 7);
//! ```
//!

// Shared<T> is Arc<T> in thread-safe mode, Rc<T> otherwise.
#[cfg(not(feature = "thread-safe"))]
pub use std::rc::Rc as Shared;
#[cfg(feature = "thread-safe")]
pub use std::sync::Arc as Shared;

/// Trait to convert provider return types into `Shared<T>`.
///
/// This trait is implemented for `Shared<U>` (i.e., `Arc<U>` or `Rc<U>`) where `U` can be
/// unsized (such as trait objects). This allows seamless conversion from a concrete or trait
/// object pointer to the unified `Shared<T>` type used by the container.
///
/// Providers that return concrete `U` can be registered via container helper methods, which
/// will perform the `Arc::new(u)` / `Rc::new(u)` wrapping as needed.
///
/// # Example
///
/// ```rust
/// use sadi::{Shared, IntoShared};
/// use std::sync::Arc;
///
/// trait Foo { fn foo(&self) -> i32; }
///
/// struct Bar;
///
/// impl Foo for Bar { fn foo(&self) -> i32 { 7 } }
///
/// let arc: Arc<dyn Foo> = Arc::new(Bar);
/// let shared: Shared<dyn Foo> = arc.into_shared();
///
/// assert_eq!(shared.foo(), 7);
/// ```
pub trait IntoShared<T: ?Sized + 'static> {
    fn into_shared(self) -> Shared<T>;
}

#[cfg(feature = "thread-safe")]
mod shared_impl_ts {
    use super::*;
    use std::sync::Arc;

    // Allow Arc<U> where U may be unsized (e.g. dyn Trait).
    impl<T: ?Sized + 'static, U: ?Sized + 'static> IntoShared<T> for Arc<U>
    where
        Arc<U>: Into<Arc<T>>,
    {
        fn into_shared(self) -> Shared<T> {
            self.into()
        }
    }
}

#[cfg(not(feature = "thread-safe"))]
mod shared_impl_nts {
    use super::*;
    use std::rc::Rc;

    // Allow Rc<U> where U may be unsized (e.g. dyn Trait).
    impl<T: ?Sized + 'static, U: ?Sized + 'static> IntoShared<T> for Rc<U>
    where
        Rc<U>: Into<Rc<T>>,
    {
        fn into_shared(self) -> Shared<T> {
            self.into()
        }
    }
}
