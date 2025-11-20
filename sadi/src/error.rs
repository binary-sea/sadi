use core::fmt;

/// Error kinds for SaDi dependency injection operations.
///
/// This enum represents all possible error conditions that can occur
/// during dependency injection operations. Each variant provides specific
/// context about what went wrong.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Service factory not registered for the requested type.
    ///
    /// This occurs when trying to resolve a service that hasn't been
    /// registered with the container using `factory()` or `factory_singleton()`.
    ServiceNotRegistered,

    /// Factory returned a value of the wrong type.
    ///
    /// This should rarely occur due to Rust's type system, but can happen
    /// if there are issues with type erasure or unsafe code.
    TypeMismatch,

    /// Cached singleton instance has wrong type.
    ///
    /// Similar to TypeMismatch, this indicates an internal error where
    /// a cached singleton doesn't match the expected type.
    CachedTypeMismatch,

    /// Factory already registered for this type.
    ///
    /// Occurs when attempting to register a factory for a type that
    /// already has a factory registered. Use the non-panicking `try_*`
    /// methods to handle this gracefully.
    FactoryAlreadyRegistered,

    /// Circular dependency detected in the dependency graph.
    ///
    /// This happens when services have dependencies that form a cycle,
    /// such as A depends on B, B depends on C, and C depends on A.
    /// The error message will include the full dependency chain.
    CircularDependency,
}

/// Error structure for SaDi dependency injection operations.
///
/// This structure combines an [`ErrorKind`] with a human-readable message
/// to provide comprehensive error information. When the `tracing` feature
/// is enabled, errors are automatically logged at appropriate levels.
///
/// # Examples
///
/// ```rust
/// use sadi::{Error, ErrorKind};
///
/// let error = Error::service_not_registered("MyService", "transient");
/// println!("Error: {}", error);
/// // Output: Error: (ServiceNotRegistered) - No transient factory registered for type: MyService
/// ```
#[derive(Debug, Clone)]
pub struct Error {
    /// The kind of error that occurred
    pub kind: ErrorKind,
    /// Human-readable error message with context
    pub message: String,
}

impl Error {
    /// Create a new SaDi error with the specified kind and message.
    ///
    /// This constructor also handles automatic logging when the `tracing`
    /// feature is enabled. Warning-level errors (like duplicate registrations)
    /// are logged as warnings, while other errors are logged as errors.
    ///
    /// # Arguments
    ///
    /// * `kind` - The type of error that occurred
    /// * `message` - A descriptive error message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::{Error, ErrorKind};
    ///
    /// let error = Error::new(
    ///     ErrorKind::ServiceNotRegistered,
    ///     "MyService not found"
    /// );
    /// ```
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        let error = Self {
            kind: kind.clone(),
            message: message.into(),
        };

        #[cfg(feature = "tracing")]
        if matches!(
            kind,
            ErrorKind::FactoryAlreadyRegistered | ErrorKind::ServiceNotRegistered
        ) {
            use tracing::warn;

            warn!("{}", error);
        } else {
            use tracing::error;

            error!("{}", error);
        }

        error
    }

    /// Create a service not registered error.
    ///
    /// This is a convenience constructor for creating errors when a requested
    /// service type has no registered factory in the container.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type that wasn't registered
    /// * `service_type` - Either "transient" or "singleton"
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::Error;
    ///
    /// let error = Error::service_not_registered("MyService", "transient");
    /// println!("{}", error);
    /// // Output: (ServiceNotRegistered) - No transient factory registered for type: MyService
    /// ```
    pub fn service_not_registered(type_name: &str, service_type: &str) -> Self {
        Self::new(
            ErrorKind::ServiceNotRegistered,
            format!(
                "No {} factory registered for type: {}",
                service_type, type_name
            ),
        )
    }

    /// Create a type mismatch error.
    ///
    /// This error occurs when a factory function returns a value that cannot
    /// be cast to the expected type. This should be rare due to Rust's type
    /// system, but can happen with type erasure issues.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type that had a mismatch
    pub fn type_mismatch(type_name: &str) -> Self {
        Self::new(
            ErrorKind::TypeMismatch,
            format!("Factory returned wrong type for: {}", type_name),
        )
    }

    /// Create a cached type mismatch error.
    ///
    /// This error occurs when a cached singleton instance cannot be cast
    /// to the expected type. This indicates an internal error in the
    /// container's caching mechanism.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type that had a cached mismatch
    pub fn cached_type_mismatch(type_name: &str) -> Self {
        Self::new(
            ErrorKind::CachedTypeMismatch,
            format!("Cached instance has wrong type for: {}", type_name),
        )
    }

    /// Create a factory already registered error.
    ///
    /// This error occurs when attempting to register a factory for a type
    /// that already has a factory registered. Use the `try_*` methods to
    /// handle this case gracefully.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type that was already registered
    /// * `service_type` - Either "transient" or "singleton"
    pub fn factory_already_registered(type_name: &str, service_type: &str) -> Self {
        Self::new(
            ErrorKind::FactoryAlreadyRegistered,
            format!(
                "{} factory already registered for type: {}",
                service_type, type_name
            ),
        )
    }

    /// Create a circular dependency error.
    ///
    /// This error occurs when the dependency graph contains a cycle, such as
    /// Service A depending on Service B, which depends on Service A. The error
    /// message includes the full dependency chain for debugging.
    ///
    /// # Arguments
    ///
    /// * `dependency_chain` - The chain of dependencies that form the cycle
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::Error;
    ///
    /// let chain = &["ServiceA", "ServiceB", "ServiceA"];
    /// let error = Error::circular_dependency(chain);
    /// println!("{}", error);
    /// // Output: (CircularDependency) - Circular dependency detected: ServiceA -> ServiceB -> ServiceA
    /// ```
    pub fn circular_dependency(dependency_chain: &[&str]) -> Self {
        Self::new(
            ErrorKind::CircularDependency,
            format!(
                "Circular dependency detected: {}",
                dependency_chain.join(" -> ")
            ),
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?}) - {}", self.kind, self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_not_registered_error() {
        let err = Error::service_not_registered("MyType", "singleton");
        assert_eq!(err.kind, ErrorKind::ServiceNotRegistered);
        assert!(err.message.contains("MyType"));
        assert!(err.message.contains("singleton"));
    }

    #[test]
    fn type_mismatch_error() {
        let err = Error::type_mismatch("OtherType");
        assert_eq!(err.kind, ErrorKind::TypeMismatch);
        assert!(err.message.contains("OtherType"));
    }

    #[test]
    fn cached_type_mismatch_error() {
        let err = Error::cached_type_mismatch("CachedType");
        assert_eq!(err.kind, ErrorKind::CachedTypeMismatch);
        assert!(err.message.contains("CachedType"));
    }

    #[test]
    fn factory_already_registered_error() {
        let err = Error::factory_already_registered("Foo", "transient");
        assert_eq!(err.kind, ErrorKind::FactoryAlreadyRegistered);
        assert!(err.message.contains("Foo"));
        assert!(err.message.contains("transient"));
    }

    #[test]
    fn circular_dependency_error() {
        let chain = ["A", "B", "A"];
        let err = Error::circular_dependency(&chain);
        assert_eq!(err.kind, ErrorKind::CircularDependency);
        assert!(err.message.contains("A -> B -> A"));
    }

    #[test]
    fn display_trait() {
        let err = Error::service_not_registered("X", "transient");
        let s = format!("{}", err);
        assert!(s.contains("ServiceNotRegistered"));
        assert!(s.contains("X"));
    }
}
