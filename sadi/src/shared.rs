// Shared<T> is Arc<T> in thread-safe mode, Rc<T> otherwise.
#[cfg(feature = "thread-safe")]
pub use std::sync::Arc as Shared;

#[cfg(not(feature = "thread-safe"))]
pub use std::rc::Rc as Shared;

/// Trait to convert provider return types into Shared<T>.
/// We implement IntoShared<T> for Shared<U> (Arc<U>/Rc<U>) only to avoid
/// overlapping impls on stable Rust. Providers that return concrete U can be
/// registered via Container helper methods (bind_value / bind_impl_as) which
/// will perform the Arc::new(u) / Rc::new(u) wrapping as needed.
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