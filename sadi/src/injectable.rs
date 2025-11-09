use std::any::Any;

/// A trait for types that can be injected by the SaDi container.
///
/// This trait should be implemented by any type that wants to be used
/// with dependency injection. It provides the necessary type information
/// and conversion methods for the container to work with trait objects.
pub trait Injectable: Any {
    /// Get a reference to the underlying type as a trait object.
    /// This is used internally by the container for type storage.
    fn as_any(&self) -> &dyn Any;

    /// Get a mutable reference to the underlying type as a trait object.
    /// This is used internally by the container for type storage.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Convert this trait object into a boxed Any.
    /// This is used internally by the container for type storage.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

/// Automatically implement Injectable for any type that implements the necessary bounds.
impl<T: Any> Injectable for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// A trait alias for types that can be registered in the container.
/// This combines Injectable with 'static to ensure proper type storage.
pub trait InjectableType: Injectable + 'static {}
impl<T: Injectable + 'static> InjectableType for T {}