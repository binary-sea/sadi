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
