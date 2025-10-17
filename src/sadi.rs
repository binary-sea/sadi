use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::Rc,
};

#[cfg(feature = "tracing")]
use tracing::{debug, error, info, trace, warn};

/// Error kinds for SaDi dependency injection operations
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Service factory not registered
    ServiceNotRegistered,
    /// Factory returned wrong type
    TypeMismatch,
    /// Cached instance has wrong type
    CachedTypeMismatch,
    /// Factory already registered
    FactoryAlreadyRegistered,
    /// Circular dependency detected
    CircularDependency,
}

/// Error structure for SaDi operations
#[derive(Debug, Clone)]
pub struct Error {
    /// The kind of error that occurred
    pub kind: ErrorKind,
    /// Human-readable error message
    pub message: String,
}

impl Error {
    /// Create a new SaDiError
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        let error = Self {
            kind,
            message: message.into(),
        };

        #[cfg(feature = "tracing")]
        {
            if matches!(
                kind,
                ErrorKind::FactoryAlreadyRegistered | ErrorKind::ServiceNotRegistered
            ) {
                warn!("{}", error);
            } else {
                error!("{}", error);
            }
        };

        error
    }

    /// Create a service not registered error
    pub fn service_not_registered(type_name: &str, service_type: &str) -> Self {
        Self::new(
            ErrorKind::ServiceNotRegistered,
            format!(
                "No {} factory registered for type: {}",
                service_type, type_name
            ),
        )
    }

    /// Create a type mismatch error
    pub fn type_mismatch(type_name: &str) -> Self {
        Self::new(
            ErrorKind::TypeMismatch,
            format!("Factory returned wrong type for: {}", type_name),
        )
    }

    /// Create a cached type mismatch error
    pub fn cached_type_mismatch(type_name: &str) -> Self {
        Self::new(
            ErrorKind::CachedTypeMismatch,
            format!("Cached instance has wrong type for: {}", type_name),
        )
    }

    /// Create a factory already registered error
    pub fn factory_already_registered(type_name: &str, service_type: &str) -> Self {
        Self::new(
            ErrorKind::FactoryAlreadyRegistered,
            format!(
                "{} factory already registered for type: {}",
                service_type, type_name
            ),
        )
    }

    /// Create a circular dependency error
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

/// A simple, flexible dependency injection container
///
/// Supports both transient and singleton service registration
/// with a clean, type-safe API.
pub struct SaDi {
    /// Factories for transient services (new instance each time)
    factories: HashMap<TypeId, Box<dyn Fn(&SaDi) -> Box<dyn Any>>>,
    /// Factories for singleton services (cached instances)
    singletons: HashMap<TypeId, Box<dyn Fn(&SaDi) -> Box<dyn Any>>>,
    /// Cache for singleton instances
    singleton_cache: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
    /// Stack to track current resolution chain for circular dependency detection
    resolution_stack: RefCell<Vec<(TypeId, &'static str)>>,
}

impl SaDi {
    /// Create a new DI container
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

    /// Register a transient factory
    ///
    /// Creates a new instance every time `get()` is called
    pub fn factory<T, F>(self, factory: F) -> Self
    where
        T: 'static + Any,
        F: Fn(&SaDi) -> T + 'static,
    {
        self.try_factory(factory)
            .unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to register a transient factory
    ///
    /// Returns Ok(Self) if successful, or Err if factory already exists
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

    /// Get a transient instance
    ///
    /// Returns a new instance every time
    pub fn get<T: 'static + Any>(&self) -> T {
        self.try_get().unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to get a transient instance
    ///
    /// Returns Ok(T) with a new instance if factory is registered, or Err with error message
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

    /// Register a singleton factory
    ///
    /// Creates the instance once and caches it for subsequent calls
    pub fn factory_singleton<T, F>(self, factory: F) -> Self
    where
        T: 'static + Any,
        F: Fn(&SaDi) -> T + 'static,
    {
        self.try_factory_singleton(factory)
            .unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to register a singleton factory
    ///
    /// Returns Ok(Self) if successful, or Err if factory already exists
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

    /// Get a singleton instance
    ///
    /// Returns the same cached instance every time
    pub fn get_singleton<T: 'static + Any>(&self) -> Rc<T> {
        self.try_get_singleton()
            .unwrap_or_else(|err| panic!("{}", err))
    }

    /// Try to get a singleton instance
    ///
    /// Returns Ok(Rc<T>) with the cached instance if factory is registered, or Err with error message
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
