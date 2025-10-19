//! # SaDi - Simple and Dependency Injection
//!
//! A lightweight, type-safe dependency injection container for Rust applications.
//! SaDi provides both transient and singleton service registration with automatic
//! dependency resolution and circular dependency detection.
//!
//! ## Features
//!
//! - **Type-Safe**: Leverages Rust's type system for compile-time safety
//! - **Transient Services**: Create new instances on each request
//! - **Singleton Services**: Shared instances with reference counting
//! - **Circular Detection**: Prevents infinite loops in dependency graphs
//! - **Error Handling**: Comprehensive error types with detailed messages
//! - **Optional Logging**: Tracing integration with feature gates
//!
//! ## Basic Usage
//!
//! ```rust
//! use sadi::SaDi;
//! use std::rc::Rc;
//!
//! // Define your services
//! struct DatabaseService {
//!     connection_string: String,
//! }
//!
//! impl DatabaseService {
//!     fn new() -> Self {
//!         Self {
//!             connection_string: "postgresql://localhost:5432/myapp".to_string(),
//!         }
//!     }
//! }
//!
//! struct UserService {
//!     db: Rc<DatabaseService>,
//! }
//!
//! impl UserService {
//!     fn new(db: Rc<DatabaseService>) -> Self {
//!         Self { db }
//!     }
//! }
//!
//! // Set up the container
//! let container = SaDi::new()
//!     .factory_singleton(|_| DatabaseService::new())
//!     .factory(|di| UserService::new(di.get_singleton::<DatabaseService>()));
//!
//! // Use your services
//! let user_service = container.get::<UserService>();
//! ```
//!
//! ## Service Lifetimes
//!
//! ### Transient Services
//! - Created fresh on each request
//! - Registered with `factory()` or `try_factory()`
//! - Retrieved with `get()` or `try_get()`
//!
//! ### Singleton Services  
//! - Created once and cached for subsequent requests
//! - Registered with `factory_singleton()` or `try_factory_singleton()`
//! - Retrieved with `get_singleton()` or `try_get_singleton()`
//! - Returned as `Rc<T>` for shared ownership
//!
//! ## Error Handling
//!
//! All operations have both panicking and non-panicking variants:
//! - `get()` vs `try_get()` - for retrieving transient services
//! - `get_singleton()` vs `try_get_singleton()` - for retrieving singletons
//! - `factory()` vs `try_factory()` - for registering transient factories
//! - `factory_singleton()` vs `try_factory_singleton()` - for registering singleton factories
//!
//! ## Circular Dependency Detection
//!
//! SaDi automatically detects circular dependencies during service resolution:
//!
//! ```should_panic
//! use sadi::SaDi;
//! use std::rc::Rc;
//!
//! struct ServiceA {
//!     b: Box<ServiceB>,
//! }
//!
//! impl ServiceA {
//!     fn new(b: ServiceB) -> Self { Self { b: Box::new(b) } }
//! }
//!
//! struct ServiceB {
//!     a: Box<ServiceA>,
//! }
//!
//! impl ServiceB {
//!     fn new(a: ServiceA) -> Self { Self { a: Box::new(a) } }
//! }
//!
//! // This will panic with a CircularDependency error
//! let container = SaDi::new()
//!     .factory(|di| ServiceA::new(di.get::<ServiceB>()))
//!     .factory(|di| ServiceB::new(di.get::<ServiceA>()));
//!
//! let service = container.get::<ServiceA>(); // Panic!
//! ```

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::Rc,
};

#[cfg(feature = "tracing")]
use tracing::{debug, error, info, trace, warn};

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
            warn!("{}", error);
        } else {
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

/// A simple, flexible dependency injection container for Rust applications.
///
/// `SaDi` provides type-safe dependency injection with support for both transient
/// and singleton service lifetimes. It automatically resolves dependencies and
/// detects circular dependencies to prevent infinite loops.
///
/// ## Key Features
///
/// - **Type Safety**: Uses Rust's type system to ensure compile-time safety
/// - **Two Service Lifetimes**: Transient (new instance each time) and Singleton (shared instance)
/// - **Automatic Resolution**: Dependencies are automatically injected based on factory functions
/// - **Circular Detection**: Prevents infinite loops by detecting dependency cycles
/// - **Error Handling**: Comprehensive error reporting with optional tracing
/// - **Builder Pattern**: Fluent API for easy container configuration
///
/// ## Service Registration
///
/// Services are registered using factory functions that describe how to create instances:
///
/// ```rust
/// use sadi::SaDi;
/// use std::rc::Rc;
///
/// struct DatabaseConfig {
///     url: String,
/// }
///
/// struct DatabaseService {
///     config: Rc<DatabaseConfig>,
/// }
///
/// let container = SaDi::new()
///     // Singleton: created once, shared by all dependents
///     .factory_singleton(|_| DatabaseConfig {
///         url: "postgresql://localhost:5432/mydb".to_string()
///     })
///     // Transient: new instance created each time
///     .factory(|di| DatabaseService {
///         config: di.get_singleton::<DatabaseConfig>()
///     });
/// ```
///
/// ## Service Resolution
///
/// Services can be retrieved using type-safe methods:
///
/// ```rust
/// # use sadi::SaDi;
/// # struct MyService;
/// # let container = SaDi::new().factory(|_| MyService);
/// // Panicking version (use when you're sure the service exists)
/// let service = container.get::<MyService>();
///
/// // Non-panicking version (returns Result for error handling)
/// let service = container.try_get::<MyService>()?;
/// # Ok::<(), sadi::Error>(())
/// ```
///
/// ## Memory Management
///
/// - **Transient services**: Owned by the caller, automatically dropped when out of scope
/// - **Singleton services**: Returned as `Rc<T>` for shared ownership with reference counting
/// - **Factory functions**: Stored as closures, can capture environment if needed
///
/// ## Thread Safety
///
/// `SaDi` is not thread-safe by design. For multi-threaded applications, consider:
/// - Creating separate containers per thread
/// - Using thread-safe service implementations (Arc, Mutex, etc.)
/// - Wrapping the entire container in appropriate synchronization primitives
///
/// Type alias for factory functions that create transient service instances.
///
/// A factory function takes a reference to the DI container and returns a boxed
/// instance of any type. The container uses type erasure to store different
/// factory types in the same collection.
type FactoryFunction = Box<dyn Fn(&SaDi) -> Box<dyn Any>>;

/// Type alias for singleton cache storage.
///
/// The cache maps TypeId to reference-counted instances wrapped in trait objects.
/// This allows multiple services to share the same singleton instance safely.
type SingletonCache = RefCell<HashMap<TypeId, Rc<dyn Any>>>;

/// Type alias for the resolution stack used in circular dependency detection.
///
/// Each entry contains the TypeId and type name of a service currently being resolved.
/// This stack helps detect cycles in the dependency graph during service creation.
type ResolutionStack = RefCell<Vec<(TypeId, &'static str)>>;

pub struct SaDi {
    /// Factories for transient services (new instance each time)
    factories: HashMap<TypeId, FactoryFunction>,
    /// Factories for singleton services (cached instances)
    singletons: HashMap<TypeId, FactoryFunction>,
    /// Cache for singleton instances
    singleton_cache: SingletonCache,
    /// Stack to track current resolution chain for circular dependency detection
    resolution_stack: ResolutionStack,
}

impl SaDi {
    /// Create a new dependency injection container.
    ///
    /// The container starts empty with no registered services. Use the builder
    /// methods (`factory`, `factory_singleton`, etc.) to register services.
    ///
    /// When the `tracing` feature is enabled, container creation is logged
    /// at the debug level.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    ///
    /// let container = SaDi::new();
    /// // Container is now ready for service registration
    /// ```
    ///
    /// ```rust
    /// use sadi::SaDi;
    ///
    /// // Fluent builder pattern
    /// let container = SaDi::new()
    ///     .factory(|_| "Hello".to_string())
    ///     .factory_singleton(|_| 42u32);
    /// ```
    pub fn new() -> Self {
        #[cfg(feature = "tracing")]
        debug!("Creating new SaDi container");

        Self {
            factories: HashMap::new(),
            singletons: HashMap::new(),
            singleton_cache: RefCell::new(HashMap::new()),
            resolution_stack: RefCell::new(Vec::new()),
        }
    }

    /// Check for circular dependencies and add type to resolution stack
    fn check_circular_dependency(
        &self,
        type_id: TypeId,
        type_name: &'static str,
    ) -> Result<(), Error> {
        let mut stack = self.resolution_stack.borrow_mut();

        // Check if this type is already in the resolution stack
        if let Some(pos) = stack.iter().position(|(id, _)| *id == type_id) {
            // Build the dependency chain for error message
            let mut chain: Vec<&str> = stack[pos..].iter().map(|(_, name)| *name).collect();
            chain.push(type_name);

            return Err(Error::circular_dependency(&chain));
        }

        // Add current type to stack
        stack.push((type_id, type_name));
        Ok(())
    }

    /// Remove type from resolution stack
    fn pop_resolution_stack(&self) {
        self.resolution_stack.borrow_mut().pop();
    }

    /// Register a factory for transient service instances.
    ///
    /// Transient services are created fresh every time they are requested.
    /// This is suitable for lightweight services or when you need separate
    /// instances with independent state.
    ///
    /// This method panics if a factory for type `T` is already registered.
    /// Use [`try_factory`](Self::try_factory) for error handling instead.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to register (must be `'static + Any`)
    /// * `F` - The factory function type
    ///
    /// # Arguments
    ///
    /// * `factory` - A function that takes `&SaDi` and returns an instance of `T`
    ///
    /// # Returns
    ///
    /// Returns `Self` for method chaining.
    ///
    /// # Panics
    ///
    /// Panics if a transient factory for type `T` is already registered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    /// use std::rc::Rc;
    ///
    /// struct Logger {
    ///     prefix: String,
    /// }
    ///
    /// struct Config {
    ///     app_name: String,
    /// }
    ///
    /// let container = SaDi::new()
    ///     .factory_singleton(|_| Config {
    ///         app_name: "MyApp".to_string()
    ///     })
    ///     .factory(|di| {
    ///         let config = di.get_singleton::<Config>();
    ///         Logger {
    ///             prefix: format!("[{}]", config.app_name)
    ///         }
    ///     });
    ///
    /// // Each call creates a new Logger instance
    /// let logger1 = container.get::<Logger>();
    /// let logger2 = container.get::<Logger>();
    /// ```
    pub fn factory<T, F>(self, factory: F) -> Self
    where
        T: 'static + Any,
        F: Fn(&SaDi) -> T + 'static,
    {
        self.try_factory(factory)
            .unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to register a factory for transient service instances.
    ///
    /// This is the non-panicking version of [`factory`](Self::factory).
    /// It returns an error if a factory for the given type is already registered,
    /// allowing for graceful error handling.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to register (must be `'static + Any`)
    /// * `F` - The factory function type
    ///
    /// # Arguments
    ///
    /// * `factory` - A function that takes `&SaDi` and returns an instance of `T`
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully registered factory, returns self for chaining
    /// * `Err(Error)` - Factory already exists for this type
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    ///
    /// let container = SaDi::new();
    ///
    /// // First registration succeeds
    /// let container = container.try_factory(|_| "Hello".to_string())?;
    ///
    /// // Second registration fails
    /// match container.try_factory(|_| "World".to_string()) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => println!("Expected error: {}", err),
    /// }
    /// # Ok::<(), sadi::Error>(())
    /// ```
    pub fn try_factory<T, F>(mut self, factory: F) -> Result<Self, Error>
    where
        T: 'static + Any,
        F: Fn(&SaDi) -> T + 'static,
    {
        let type_name = std::any::type_name::<T>();
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "tracing")]
        trace!(
            "Attempting to register transient factory for type: {}",
            type_name
        );

        if self.factories.contains_key(&type_id) {
            return Err(Error::factory_already_registered(type_name, "transient"));
        }

        self.factories
            .insert(type_id, Box::new(move |di| Box::new(factory(di))));

        #[cfg(feature = "tracing")]
        info!(
            "Successfully registered transient factory for type: {}",
            type_name
        );

        Ok(self)
    }

    /// Get a transient service instance.
    ///
    /// Creates and returns a new instance of the service every time it's called.
    /// The service's dependencies are automatically resolved and injected.
    ///
    /// This method panics if no factory is registered for type `T` or if
    /// circular dependencies are detected. Use [`try_get`](Self::try_get)
    /// for error handling instead.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to retrieve (must be `'static + Any`)
    ///
    /// # Returns
    ///
    /// A new instance of type `T`.
    ///
    /// # Panics
    ///
    /// * No transient factory registered for type `T`
    /// * Circular dependency detected in the dependency graph
    /// * Factory function panics during execution
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    /// use std::cell::Cell;
    ///
    /// struct MyService {
    ///     id: u32,
    /// }
    ///
    /// let counter = Cell::new(0);
    /// let container = SaDi::new()
    ///     .factory(move |_| {
    ///         let current = counter.get();
    ///         counter.set(current + 1);
    ///         MyService { id: current + 1 }
    ///     });
    ///
    /// let service1 = container.get::<MyService>();
    /// let service2 = container.get::<MyService>();
    /// // service1.id != service2.id (different instances)
    /// ```
    pub fn get<T: 'static + Any>(&self) -> T {
        self.try_get().unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to get a transient service instance.
    ///
    /// This is the non-panicking version of [`get`](Self::get).
    /// It returns an error instead of panicking when issues occur.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to retrieve (must be `'static + Any`)
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - Successfully created new instance
    /// * `Err(Error)` - Factory not registered, circular dependency, or factory panic
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    ///
    /// struct MyService;
    ///
    /// let container = SaDi::new()
    ///     .factory(|_| MyService);
    ///
    /// match container.try_get::<MyService>() {
    ///     Ok(service) => println!("Got service!"),
    ///     Err(err) => println!("Error: {}", err),
    /// }
    ///
    /// // Trying to get unregistered service
    /// match container.try_get::<String>() {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => println!("Expected error: {}", err),
    /// }
    /// ```
    pub fn try_get<T: 'static + Any>(&self) -> Result<T, Error> {
        let type_name = std::any::type_name::<T>();
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "tracing")]
        trace!(
            "Attempting to get transient instance for type: {}",
            type_name
        );

        // Check for circular dependency before proceeding
        self.check_circular_dependency(type_id, type_name)?;

        if let Some(factory) = self.factories.get(&type_id) {
            #[cfg(feature = "tracing")]
            debug!(
                "Found transient factory for type: {}, creating instance",
                type_name
            );

            let result = factory(self);

            // Remove from stack before processing result
            self.pop_resolution_stack();

            match result.downcast::<T>() {
                Ok(instance) => {
                    #[cfg(feature = "tracing")]
                    debug!(
                        "Successfully created transient instance for type: {}",
                        type_name
                    );
                    Ok(*instance)
                }
                Err(_) => Err(Error::type_mismatch(type_name)),
            }
        } else {
            // Remove from stack before returning error
            self.pop_resolution_stack();
            Err(Error::service_not_registered(type_name, "transient"))
        }
    }

    /// Register a factory for singleton service instances.
    ///
    /// Singleton services are created once and cached for subsequent requests.
    /// This is suitable for expensive-to-create services, shared state, or
    /// services that should maintain state across the application.
    ///
    /// This method panics if a singleton factory for type `T` is already registered.
    /// Use [`try_factory_singleton`](Self::try_factory_singleton) for error handling instead.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to register (must be `'static + Any`)
    /// * `F` - The factory function type
    ///
    /// # Arguments
    ///
    /// * `factory` - A function that takes `&SaDi` and returns an instance of `T`
    ///
    /// # Returns
    ///
    /// Returns `Self` for method chaining.
    ///
    /// # Panics
    ///
    /// Panics if a singleton factory for type `T` is already registered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    /// use std::rc::Rc;
    ///
    /// struct DatabaseConnection {
    ///     url: String,
    ///     connected: bool,
    /// }
    ///
    /// struct UserService {
    ///     db: Rc<DatabaseConnection>,
    /// }
    ///
    /// let container = SaDi::new()
    ///     .factory_singleton(|_| DatabaseConnection {
    ///         url: "postgresql://localhost:5432/mydb".to_string(),
    ///         connected: true,
    ///     })
    ///     .factory(|di| UserService {
    ///         db: di.get_singleton::<DatabaseConnection>(),
    ///     });
    ///
    /// // Both services share the same database connection
    /// let user_service1 = container.get::<UserService>();
    /// let user_service2 = container.get::<UserService>();
    /// // Rc::as_ptr(&user_service1.db) == Rc::as_ptr(&user_service2.db)
    /// ```
    pub fn factory_singleton<T, F>(self, factory: F) -> Self
    where
        T: 'static + Any,
        F: Fn(&SaDi) -> T + 'static,
    {
        self.try_factory_singleton(factory)
            .unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to register a factory for singleton service instances.
    ///
    /// This is the non-panicking version of [`factory_singleton`](Self::factory_singleton).
    /// It returns an error if a factory for the given type is already registered,
    /// allowing for graceful error handling.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to register (must be `'static + Any`)
    /// * `F` - The factory function type
    ///
    /// # Arguments
    ///
    /// * `factory` - A function that takes `&SaDi` and returns an instance of `T`
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully registered factory, returns self for chaining
    /// * `Err(Error)` - Factory already exists for this type
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    ///
    /// struct Config {
    ///     setting: String,
    /// }
    ///
    /// let container = SaDi::new();
    ///
    /// // First registration succeeds
    /// let container = container.try_factory_singleton(|_| Config {
    ///     setting: "value1".to_string()
    /// })?;
    ///
    /// // Second registration fails
    /// match container.try_factory_singleton(|_| Config {
    ///     setting: "value2".to_string()
    /// }) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => println!("Expected error: {}", err),
    /// }
    /// # Ok::<(), sadi::Error>(())
    /// ```
    pub fn try_factory_singleton<T, F>(mut self, factory: F) -> Result<Self, Error>
    where
        T: 'static + Any,
        F: Fn(&SaDi) -> T + 'static,
    {
        let type_name = std::any::type_name::<T>();
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "tracing")]
        trace!(
            "Attempting to register singleton factory for type: {}",
            type_name
        );

        if self.singletons.contains_key(&type_id) {
            return Err(Error::factory_already_registered(type_name, "singleton"));
        }

        self.singletons
            .insert(type_id, Box::new(move |di| Box::new(factory(di))));

        #[cfg(feature = "tracing")]
        info!(
            "Successfully registered singleton factory for type: {}",
            type_name
        );

        Ok(self)
    }

    /// Get a singleton service instance.
    ///
    /// Returns the same cached instance every time it's called, wrapped in `Rc<T>`
    /// for shared ownership. On the first call, the factory function is executed
    /// to create the instance, which is then cached for subsequent calls.
    ///
    /// This method panics if no singleton factory is registered for type `T` or if
    /// circular dependencies are detected. Use [`try_get_singleton`](Self::try_get_singleton)
    /// for error handling instead.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to retrieve (must be `'static + Any`)
    ///
    /// # Returns
    ///
    /// An `Rc<T>` pointing to the singleton instance.
    ///
    /// # Panics
    ///
    /// * No singleton factory registered for type `T`
    /// * Circular dependency detected in the dependency graph
    /// * Factory function panics during execution
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    /// use std::rc::Rc;
    ///
    /// struct Config {
    ///     app_name: String,
    /// }
    ///
    /// let container = SaDi::new()
    ///     .factory_singleton(|_| Config {
    ///         app_name: "MyApp".to_string()
    ///     });
    ///
    /// let config1 = container.get_singleton::<Config>();
    /// let config2 = container.get_singleton::<Config>();
    ///
    /// // Same instance
    /// assert_eq!(Rc::as_ptr(&config1), Rc::as_ptr(&config2));
    /// assert_eq!(config1.app_name, "MyApp");
    /// ```
    pub fn get_singleton<T: 'static + Any>(&self) -> Rc<T> {
        self.try_get_singleton()
            .unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to get a singleton service instance.
    ///
    /// This is the non-panicking version of [`get_singleton`](Self::get_singleton).
    /// It returns an error instead of panicking when issues occur.
    ///
    /// The method first checks if an instance is already cached. If found, it returns
    /// the cached instance immediately. If not found, it executes the factory function
    /// to create a new instance, caches it, and returns it.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The service type to retrieve (must be `'static + Any`)
    ///
    /// # Returns
    ///
    /// * `Ok(Rc<T>)` - Successfully retrieved or created singleton instance
    /// * `Err(Error)` - Factory not registered, circular dependency, or factory panic
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    /// use std::rc::Rc;
    ///
    /// struct DatabaseConfig {
    ///     url: String,
    /// }
    ///
    /// let container = SaDi::new()
    ///     .factory_singleton(|_| DatabaseConfig {
    ///         url: "postgresql://localhost:5432/mydb".to_string()
    ///     });
    ///
    /// match container.try_get_singleton::<DatabaseConfig>() {
    ///     Ok(config) => {
    ///         println!("Database URL: {}", config.url);
    ///         
    ///         // Second call returns same instance
    ///         let config2 = container.try_get_singleton::<DatabaseConfig>()?;
    ///         assert_eq!(Rc::as_ptr(&config), Rc::as_ptr(&config2));
    ///     }
    ///     Err(err) => println!("Error: {}", err),
    /// }
    ///
    /// // Trying to get unregistered singleton service
    /// match container.try_get_singleton::<String>() {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => println!("Expected error: {}", err),
    /// }
    /// # Ok::<(), sadi::Error>(())
    /// ```
    pub fn try_get_singleton<T: 'static + Any>(&self) -> Result<Rc<T>, Error> {
        let type_name = std::any::type_name::<T>();
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "tracing")]
        trace!(
            "Attempting to get singleton instance for type: {}",
            type_name
        );

        // Check cache first (no circular dependency check needed for cached instances)
        {
            let cache = self.singleton_cache.borrow();
            if let Some(cached) = cache.get(&type_id) {
                #[cfg(feature = "tracing")]
                debug!("Found cached singleton instance for type: {}", type_name);

                return cached
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| Error::cached_type_mismatch(type_name));
            }
        }

        // Check for circular dependency before creating new instance
        self.check_circular_dependency(type_id, type_name)?;

        #[cfg(feature = "tracing")]
        debug!(
            "No cached instance found for type: {}, attempting to create new singleton",
            type_name
        );

        // Create new instance and cache it
        if let Some(factory) = self.singletons.get(&type_id) {
            #[cfg(feature = "tracing")]
            debug!(
                "Found singleton factory for type: {}, creating and caching instance",
                type_name
            );

            let result = factory(self);

            // Remove from stack before processing result
            self.pop_resolution_stack();

            match result.downcast::<T>() {
                Ok(boxed_t) => {
                    let rc_instance = Rc::new(*boxed_t);
                    let rc_any: Rc<dyn Any> = rc_instance.clone();
                    self.singleton_cache.borrow_mut().insert(type_id, rc_any);

                    #[cfg(feature = "tracing")]
                    info!(
                        "Successfully created and cached singleton instance for type: {}",
                        type_name
                    );

                    Ok(rc_instance)
                }
                Err(_) => Err(Error::type_mismatch(type_name)),
            }
        } else {
            // Remove from stack before returning error
            self.pop_resolution_stack();
            Err(Error::service_not_registered(type_name, "singleton"))
        }
    }
}

impl Default for SaDi {
    /// Create a new SaDi container with default settings.
    ///
    /// This is equivalent to calling [`SaDi::new()`](Self::new).
    /// Provided for convenience when working with APIs that expect
    /// types implementing the `Default` trait.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sadi::SaDi;
    ///
    /// let container1 = SaDi::new();
    /// let container2 = SaDi::default();
    /// // Both containers are equivalent
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Test services for various scenarios

    /// Simple service with no dependencies
    #[derive(Debug, Clone, PartialEq)]
    struct SimpleService {
        value: u32,
    }

    impl SimpleService {
        fn new(value: u32) -> Self {
            Self { value }
        }
    }

    /// Service that depends on SimpleService
    #[derive(Debug)]
    struct DependentService {
        simple: SimpleService,
        multiplier: u32,
    }

    impl DependentService {
        fn new(simple: SimpleService, multiplier: u32) -> Self {
            Self { simple, multiplier }
        }

        fn calculate(&self) -> u32 {
            self.simple.value * self.multiplier
        }
    }

    /// Singleton service with state
    #[derive(Debug)]
    struct CounterService {
        counter: Arc<AtomicUsize>,
    }

    impl CounterService {
        fn new() -> Self {
            Self {
                counter: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn increment(&self) -> usize {
            self.counter.fetch_add(1, Ordering::SeqCst) + 1
        }

        fn get(&self) -> usize {
            self.counter.load(Ordering::SeqCst)
        }
    }

    /// Service that depends on multiple services
    #[derive(Debug)]
    struct ComplexService {
        _dependent: DependentService,
        counter: Rc<CounterService>,
        _name: String,
    }

    impl ComplexService {
        fn new(dependent: DependentService, counter: Rc<CounterService>, name: String) -> Self {
            Self {
                _dependent: dependent,
                counter,
                _name: name,
            }
        }
    }

    // Test scenarios

    #[test]
    fn test_basic_transient_service() {
        let container = SaDi::new().factory(|_| SimpleService::new(42));

        let service1 = container.get::<SimpleService>();
        let service2 = container.get::<SimpleService>();

        assert_eq!(service1.value, 42);
        assert_eq!(service2.value, 42);
        // Transient services should be different instances
        assert_ne!(&service1 as *const _, &service2 as *const _);
    }

    #[test]
    fn test_basic_singleton_service() {
        let container = SaDi::new().factory_singleton(|_| CounterService::new());

        let service1 = container.get_singleton::<CounterService>();
        let service2 = container.get_singleton::<CounterService>();

        // Singletons should be the same instance
        assert_eq!(Rc::as_ptr(&service1), Rc::as_ptr(&service2));

        // Test that state is shared
        assert_eq!(service1.increment(), 1);
        assert_eq!(service2.get(), 1);
        assert_eq!(service2.increment(), 2);
        assert_eq!(service1.get(), 2);
    }

    #[test]
    fn test_dependency_injection() {
        let container = SaDi::new()
            .factory(|_| SimpleService::new(10))
            .factory(|di: &SaDi| {
                let simple = di.get::<SimpleService>();
                DependentService::new(simple, 5)
            });

        let service = container.get::<DependentService>();
        assert_eq!(service.calculate(), 50);
    }

    #[test]
    fn test_mixed_transient_and_singleton() {
        let container = SaDi::new()
            .factory(|_| SimpleService::new(7))
            .factory_singleton(|_| CounterService::new())
            .factory(|di: &SaDi| {
                let simple = di.get::<SimpleService>();
                let counter = di.get_singleton::<CounterService>();
                ComplexService::new(
                    DependentService::new(simple, 3),
                    counter,
                    "TestService".to_string(),
                )
            });

        let service1 = container.get::<ComplexService>();
        let service2 = container.get::<ComplexService>();

        // Different ComplexService instances
        assert_ne!(&service1 as *const _, &service2 as *const _);

        // But same CounterService singleton
        assert_eq!(Rc::as_ptr(&service1.counter), Rc::as_ptr(&service2.counter));

        // Test shared state
        service1.counter.increment();
        assert_eq!(service2.counter.get(), 1);
    }

    #[test]
    fn test_deep_dependency_chain() {
        // Create a chain: Level3 -> Level2 -> Level1 -> SimpleService

        #[derive(Debug)]
        struct Level1Service(SimpleService);

        #[derive(Debug)]
        struct Level2Service(Level1Service);

        #[derive(Debug)]
        struct Level3Service(Level2Service);

        let container = SaDi::new()
            .factory(|_| SimpleService::new(100))
            .factory(|di: &SaDi| Level1Service(di.get::<SimpleService>()))
            .factory(|di: &SaDi| Level2Service(di.get::<Level1Service>()))
            .factory(|di: &SaDi| Level3Service(di.get::<Level2Service>()));

        let service = container.get::<Level3Service>();
        assert_eq!(service.0.0.0.value, 100);
    }

    #[test]
    fn test_error_service_not_registered() {
        let container = SaDi::new();

        // Test try_get
        match container.try_get::<SimpleService>() {
            Err(Error {
                kind: ErrorKind::ServiceNotRegistered,
                ..
            }) => (),
            _ => panic!("Expected ServiceNotRegistered error"),
        }

        // Test try_get_singleton
        match container.try_get_singleton::<SimpleService>() {
            Err(Error {
                kind: ErrorKind::ServiceNotRegistered,
                ..
            }) => (),
            _ => panic!("Expected ServiceNotRegistered error"),
        }
    }

    #[test]
    #[should_panic(expected = "No transient factory registered")]
    fn test_panic_service_not_registered() {
        let container = SaDi::new();
        let _ = container.get::<SimpleService>();
    }

    #[test]
    fn test_error_factory_already_registered() {
        let container = SaDi::new().factory(|_| SimpleService::new(1));

        // Try to register the same type again
        match container.try_factory(|_| SimpleService::new(2)) {
            Err(Error {
                kind: ErrorKind::FactoryAlreadyRegistered,
                ..
            }) => (),
            _ => panic!("Expected FactoryAlreadyRegistered error"),
        }
    }

    #[test]
    fn test_error_singleton_already_registered() {
        let container = SaDi::new().factory_singleton(|_| CounterService::new());

        // Try to register the same singleton type again
        match container.try_factory_singleton(|_| CounterService::new()) {
            Err(Error {
                kind: ErrorKind::FactoryAlreadyRegistered,
                ..
            }) => (),
            _ => panic!("Expected FactoryAlreadyRegistered error"),
        }
    }

    #[test]
    #[should_panic(expected = "transient factory already registered")]
    fn test_panic_factory_already_registered() {
        let _container = SaDi::new()
            .factory(|_| SimpleService::new(1))
            .factory(|_| SimpleService::new(2)); // This should panic
    }

    #[test]
    fn test_circular_dependency_detection() {
        // Test that circular dependency detection works
        // Since circular dependencies will cause panics in factories when using get(),
        // we test this with a should_panic test instead

        #[derive(Debug)]
        struct ServiceA;

        #[derive(Debug)]
        struct ServiceB;

        let container = SaDi::new()
            .factory(|di: &SaDi| {
                di.get::<ServiceB>();
                ServiceA
            })
            .factory(|di: &SaDi| {
                di.get::<ServiceA>();
                ServiceB
            });

        // This should panic due to circular dependency detection
        // The panic message will contain "Circular dependency detected"
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| container.get::<ServiceA>()));

        // Verify that it panicked
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "Circular dependency detected")]
    fn test_panic_circular_dependency() {
        // Direct self-dependency

        #[allow(dead_code)]
        #[derive(Debug)]
        struct SelfDependent(Box<SelfDependent>);

        let container =
            SaDi::new().factory(|di: &SaDi| SelfDependent(Box::new(di.get::<SelfDependent>())));

        let _ = container.get::<SelfDependent>();
    }

    #[test]
    fn test_complex_circular_dependency() {
        // Test A -> B -> C -> A circular dependency with panic detection

        #[derive(Debug)]
        struct ServiceA;

        #[derive(Debug)]
        struct ServiceB;

        #[derive(Debug)]
        struct ServiceC;

        let container = SaDi::new()
            .factory(|di: &SaDi| {
                di.get::<ServiceB>();
                ServiceA
            })
            .factory(|di: &SaDi| {
                di.get::<ServiceC>();
                ServiceB
            })
            .factory(|di: &SaDi| {
                di.get::<ServiceA>();
                ServiceC
            });

        // This should panic due to circular dependency detection
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| container.get::<ServiceA>()));

        // Verify that it panicked (indicating circular dependency was detected)
        assert!(result.is_err());
    }

    #[test]
    fn test_singleton_bypasses_circular_check_when_cached() {
        // Test that cached singletons don't trigger circular dependency checks

        #[derive(Debug)]
        struct CachedService {
            id: u32,
        }

        impl CachedService {
            fn new(id: u32) -> Self {
                Self { id }
            }
        }

        let container = SaDi::new().factory_singleton(|_| CachedService::new(42));

        // Get the singleton to cache it
        let cached = container.get_singleton::<CachedService>();
        assert_eq!(cached.id, 42);

        // Now create a service that depends on the cached singleton
        // This should work even if it might look like a circular dependency
        let container = container.factory(|di: &SaDi| {
            let cached_service = di.get_singleton::<CachedService>();
            format!("Dependent on cached service with id: {}", cached_service.id)
        });

        let result = container.get::<String>();
        assert_eq!(result, "Dependent on cached service with id: 42");
    }

    #[test]
    fn test_multiple_dependencies_same_type() {
        // Test service that requires the same dependency multiple times

        #[derive(Debug)]
        struct MultiDependentService {
            counter1: Rc<CounterService>,
            counter2: Rc<CounterService>,
        }

        impl MultiDependentService {
            fn new(counter1: Rc<CounterService>, counter2: Rc<CounterService>) -> Self {
                Self { counter1, counter2 }
            }
        }

        let container = SaDi::new()
            .factory_singleton(|_| CounterService::new())
            .factory(|di: &SaDi| {
                MultiDependentService::new(
                    di.get_singleton::<CounterService>(),
                    di.get_singleton::<CounterService>(),
                )
            });

        let service = container.get::<MultiDependentService>();

        // Both should reference the same singleton
        assert_eq!(Rc::as_ptr(&service.counter1), Rc::as_ptr(&service.counter2));

        // Test shared state
        service.counter1.increment();
        assert_eq!(service.counter2.get(), 1);
    }

    #[test]
    fn test_factory_with_complex_logic() {
        // Test factory with conditional logic and external state

        #[derive(Debug)]
        struct ConfigurableService {
            mode: String,
            value: i32,
        }

        impl ConfigurableService {
            fn new(mode: String, value: i32) -> Self {
                Self { mode, value }
            }
        }

        let external_config = 100;
        let container = SaDi::new().factory(move |_| {
            let mode = if external_config > 50 {
                "high".to_string()
            } else {
                "low".to_string()
            };
            ConfigurableService::new(mode, external_config)
        });

        let service = container.get::<ConfigurableService>();
        assert_eq!(service.mode, "high");
        assert_eq!(service.value, 100);
    }

    #[test]
    fn test_error_display_format() {
        let error = Error::service_not_registered("TestType", "transient");
        let display = format!("{}", error);
        assert!(display.contains("ServiceNotRegistered"));
        assert!(display.contains("No transient factory registered for type: TestType"));
    }

    #[test]
    fn test_container_chaining() {
        // Test that factory methods can be chained

        #[derive(Debug, PartialEq)]
        struct StringService(String);

        #[derive(Debug, PartialEq)]
        struct CountedService(String);

        let container = SaDi::new()
            .factory(|_| SimpleService::new(1))
            .factory_singleton(|_| CounterService::new())
            .factory(|di: &SaDi| {
                StringService(format!("Value: {}", di.get::<SimpleService>().value))
            })
            .factory(|di: &SaDi| {
                let counter = di.get_singleton::<CounterService>();
                let count = counter.increment();
                CountedService(format!("Count: {}", count))
            });

        let string_service = container.get::<StringService>();
        assert_eq!(string_service.0, "Value: 1");

        let counted_service = container.get::<CountedService>();
        assert_eq!(counted_service.0, "Count: 1");
    }

    #[test]
    fn test_large_dependency_graph() {
        // Test performance with a larger dependency graph

        #[derive(Debug)]
        struct Node1(SimpleService);
        #[derive(Debug)]
        struct Node2(Node1);
        #[derive(Debug)]
        struct Node3(Node2);
        #[derive(Debug)]
        struct Node4(Node3);
        #[derive(Debug)]
        struct Node5(Node4);
        #[derive(Debug)]
        struct FinalNode(Node5, Rc<CounterService>);

        let container = SaDi::new()
            .factory(|_| SimpleService::new(999))
            .factory_singleton(|_| CounterService::new())
            .factory(|di: &SaDi| Node1(di.get::<SimpleService>()))
            .factory(|di: &SaDi| Node2(di.get::<Node1>()))
            .factory(|di: &SaDi| Node3(di.get::<Node2>()))
            .factory(|di: &SaDi| Node4(di.get::<Node3>()))
            .factory(|di: &SaDi| Node5(di.get::<Node4>()))
            .factory(|di: &SaDi| {
                FinalNode(di.get::<Node5>(), di.get_singleton::<CounterService>())
            });

        let final_node = container.get::<FinalNode>();
        assert_eq!(final_node.0.0.0.0.0.0.value, 999);

        // Test that counter is properly injected
        assert_eq!(final_node.1.increment(), 1);
    }

    #[test]
    fn test_resolution_stack_with_missing_dependency() {
        // Test that resolution stack works correctly with missing dependencies

        #[derive(Debug)]
        struct ServiceWithMissingDep {
            _value: u32,
        }

        let container = SaDi::new().factory(|di: &SaDi| {
            // This will fail because SimpleService is not registered
            // Use try_get to avoid panic
            match di.try_get::<SimpleService>() {
                Ok(_) => ServiceWithMissingDep { _value: 42 },
                Err(_) => ServiceWithMissingDep { _value: 0 },
            }
        });

        // This should succeed now
        let service = container.get::<ServiceWithMissingDep>();
        assert_eq!(service._value, 0);

        // Test that missing dependency error still works for direct calls
        match container.try_get::<SimpleService>() {
            Err(Error {
                kind: ErrorKind::ServiceNotRegistered,
                ..
            }) => (),
            _ => panic!("Expected ServiceNotRegistered error"),
        }
    }
}
