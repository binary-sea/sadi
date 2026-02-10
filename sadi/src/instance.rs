//! Instance wrapper for dependency injection values.
//!
//! This module provides the [`Instance`] type, which wraps a [`Shared`] reference
//! to provide convenient access to dependency-injected values. It serves as the
//! primary interface for consuming resolved dependencies.
//!
//! # Core Responsibilities
//!
//! - **Value Access**: Provides ergonomic access to the underlying value via references
//! - **Shared Ownership**: Maintains shared ownership through reference counting
//! - **Type Safety**: Preserves type information including support for trait objects
//!
//! # Design Philosophy
//!
//! The `Instance` type is a thin wrapper around `Shared<T>` that provides a clear
//! semantic distinction between "a resolved dependency" and "a shared reference".
//! This makes the DI container's API more intuitive and self-documenting.
//!
//! # Thread Safety
//!
//! When compiled with the `thread-safe` feature, `Instance<T>` can be safely shared
//! across threads (assuming `T: Send + Sync`). The underlying `Shared` type will be
//! `Arc<T>`, providing atomic reference counting.
//!
//! # Examples
//!
//! Creating and using an instance:
//!
//! ```
//! use sadi::{Instance, Shared};
//!
//! struct Config {
//!     debug: bool,
//!     port: u16,
//! }
//!
//! let shared_config = Shared::new(Config { debug: true, port: 8080 });
//! let instance = Instance::new(shared_config);
//!
//! // Access via reference
//! assert_eq!(instance.get().port, 8080);
//!
//! // Get a cloned Shared reference
//! let shared = instance.value();
//! assert_eq!(shared.port, 8080);
//! ```
//!
//! With trait objects:
//!
//! ```
//! use sadi::{Instance, Shared};
//!
//! trait Logger {
//!     fn log(&self, message: &str);
//! }
//!
//! struct ConsoleLogger;
//! impl Logger for ConsoleLogger {
//!     fn log(&self, message: &str) {
//!         println!("{}", message);
//!     }
//! }
//!
//! let logger: Shared<dyn Logger> = Shared::new(ConsoleLogger);
//! let instance = Instance::<dyn Logger>::new(logger);
//!
//! instance.get().log("Hello, world!");
//! ```

use crate::Shared;

/// A wrapper around a shared reference to a dependency-injected value.
///
/// `Instance<T>` provides a convenient interface for accessing values resolved
/// by the dependency injection container. It maintains shared ownership of the
/// underlying value through reference counting.
///
/// # Type Parameters
///
/// - `T`: The type of the wrapped value. Can be `?Sized` to support trait objects.
///
/// # Invariants
///
/// - Always contains a valid `Shared<T>` reference
/// - The wrapped value is immutable (interior mutability requires explicit use
///   of `Mutex`, `RwLock`, `RefCell`, etc.)
///
/// # Memory Management
///
/// The instance holds a strong reference to the underlying value. The value will
/// be deallocated when all `Instance` wrappers and `Shared` references are dropped.
///
/// # Examples
///
/// Basic usage with a concrete type:
///
/// ```
/// use sadi::{Instance, Shared};
///
/// #[derive(Debug, PartialEq)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user = Shared::new(User {
///     id: 1,
///     name: "Alice".to_string(),
/// });
///
/// let instance = Instance::new(user);
/// assert_eq!(instance.get().id, 1);
/// assert_eq!(instance.get().name, "Alice");
/// ```
///
/// Multiple instances sharing the same value:
///
/// ```
/// use sadi::{Instance, Shared};
///
/// let shared = Shared::new(vec![1, 2, 3]);
/// let instance1 = Instance::new(shared.clone());
/// let instance2 = Instance::new(shared.clone());
///
/// // Both instances point to the same allocation
/// assert!(Shared::ptr_eq(&instance1.value(), &instance2.value()));
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Instance<T: ?Sized + 'static> {
    /// The shared reference to the actual value
    value: Shared<T>,
}

impl<T: ?Sized + 'static> Instance<T> {
    /// Creates a new `Instance` wrapping the given shared value.
    ///
    /// This is typically called by the dependency injection container after
    /// resolving a dependency, but can also be used directly for testing or
    /// manual dependency management.
    ///
    /// # Arguments
    ///
    /// * `value` - A `Shared<T>` reference to wrap
    ///
    /// # Examples
    ///
    /// Creating an instance with a concrete type:
    ///
    /// ```
    /// use sadi::{Instance, Shared};
    ///
    /// struct Database {
    ///     url: String,
    /// }
    ///
    /// let db = Shared::new(Database {
    ///     url: "postgresql://localhost".to_string(),
    /// });
    ///
    /// let instance = Instance::new(db);
    /// ```
    ///
    /// Creating an instance with a trait object:
    ///
    /// ```
    /// use sadi::{Instance, Shared};
    ///
    /// trait Repository {}
    /// struct UserRepository;
    /// impl Repository for UserRepository {}
    ///
    /// let repo: Shared<dyn Repository> = Shared::new(UserRepository);
    /// let instance = Instance::<dyn Repository>::new(repo);
    /// ```
    pub fn new(value: Shared<T>) -> Self {
        Self { value }
    }

    /// Returns a reference to the wrapped value.
    ///
    /// This provides direct, immutable access to the underlying value without
    /// requiring an additional clone of the `Shared` wrapper. The reference
    /// is valid for as long as the `Instance` exists.
    ///
    /// # Returns
    ///
    /// An immutable reference to the wrapped value of type `&T`.
    ///
    /// # Performance
    ///
    /// This is a zero-cost operation that simply dereferences the `Shared`
    /// pointer. No cloning or additional allocations occur.
    ///
    /// # Examples
    ///
    /// Accessing fields:
    ///
    /// ```
    /// use sadi::{Instance, Shared};
    ///
    /// struct Config {
    ///     debug: bool,
    ///     timeout_ms: u64,
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Config {
    ///     debug: true,
    ///     timeout_ms: 5000,
    /// }));
    ///
    /// assert!(instance.get().debug);
    /// assert_eq!(instance.get().timeout_ms, 5000);
    /// ```
    ///
    /// Calling methods:
    ///
    /// ```
    /// use sadi::{Instance, Shared};
    ///
    /// struct Calculator {
    ///     base: i32,
    /// }
    ///
    /// impl Calculator {
    ///     fn add(&self, x: i32) -> i32 {
    ///         self.base + x
    ///     }
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Calculator { base: 10 }));
    /// assert_eq!(instance.get().add(5), 15);
    /// ```
    ///
    /// Using with trait objects:
    ///
    /// ```
    /// use sadi::{Instance, Shared};
    ///
    /// trait Greeter {
    ///     fn greet(&self) -> String;
    /// }
    ///
    /// struct EnglishGreeter;
    /// impl Greeter for EnglishGreeter {
    ///     fn greet(&self) -> String {
    ///         "Hello!".to_string()
    ///     }
    /// }
    ///
    /// let greeter: Shared<dyn Greeter> = Shared::new(EnglishGreeter);
    /// let instance = Instance::<dyn Greeter>::new(greeter);
    ///
    /// assert_eq!(instance.get().greet(), "Hello!");
    /// ```
    pub fn get(&self) -> &T {
        &*self.value
    }

    /// Returns a clone of the underlying `Shared<T>` reference.
    ///
    /// This creates a new strong reference to the same underlying value,
    /// incrementing the reference count. The cloned `Shared` can be stored,
    /// passed to other functions, or used to create additional `Instance`
    /// wrappers.
    ///
    /// # Returns
    ///
    /// A `Shared<T>` that points to the same allocation as this instance.
    ///
    /// # Performance
    ///
    /// This operation performs a reference count increment, which is:
    /// - Atomic (when using `thread-safe` feature with `Arc`)
    /// - Non-atomic but very fast (when using `Rc` without `thread-safe`)
    ///
    /// No deep cloning of the actual value occurs.
    ///
    /// # Use Cases
    ///
    /// - **Sharing dependencies**: Pass the value to multiple components
    /// - **Storing references**: Keep a reference in a struct or collection
    /// - **Background tasks**: Send the value to async tasks or threads (with `thread-safe`)
    /// - **Testing**: Create test instances that share the same mock
    ///
    /// # Examples
    ///
    /// Storing multiple references:
    ///
    /// ```
    /// use sadi::{Instance, Shared};
    ///
    /// struct Config {
    ///     max_connections: u32,
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Config { max_connections: 100 }));
    ///
    /// let shared1 = instance.value();
    /// let shared2 = instance.value();
    ///
    /// // All point to the same allocation
    /// assert!(Shared::ptr_eq(&shared1, &shared2));
    /// assert!(Shared::ptr_eq(&shared1, &instance.value()));
    /// ```
    ///
    /// Passing to multiple services:
    ///
    /// ```
    /// use sadi::{Instance, Shared};
    ///
    /// struct Database {
    ///     url: String,
    /// }
    ///
    /// struct UserService {
    ///     db: Shared<Database>,
    /// }
    ///
    /// struct OrderService {
    ///     db: Shared<Database>,
    /// }
    ///
    /// let db_instance = Instance::new(Shared::new(Database {
    ///     url: "postgresql://localhost".to_string(),
    /// }));
    ///
    /// let user_service = UserService {
    ///     db: db_instance.value(),
    /// };
    ///
    /// let order_service = OrderService {
    ///     db: db_instance.value(),
    /// };
    ///
    /// // Both services share the same database connection
    /// assert!(Shared::ptr_eq(&user_service.db, &order_service.db));
    /// ```
    ///
    /// # Thread Safety
    ///
    /// When the `thread-safe` feature is enabled, the returned `Shared<T>`
    /// (which is `Arc<T>`) can be safely sent to other threads:
    ///
    /// ```no_run
    /// # #[cfg(feature = "thread-safe")]
    /// # {
    /// use sadi::{Instance, Shared};
    /// use std::thread;
    ///
    /// struct Counter {
    ///     value: std::sync::atomic::AtomicU32,
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Counter {
    ///     value: std::sync::atomic::AtomicU32::new(0),
    /// }));
    ///
    /// let shared = instance.value();
    /// let handle = thread::spawn(move || {
    ///     shared.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    /// });
    ///
    /// handle.join().unwrap();
    /// # }
    /// ```
    pub fn value(&self) -> Shared<T> {
        self.value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestData {
        id: u32,
        name: String,
    }

    #[derive(Debug, PartialEq)]
    struct Counter {
        value: u32,
    }

    trait Service: std::fmt::Debug {
        fn name(&self) -> &str;
        fn execute(&self) -> String;
    }

    #[derive(Debug)]
    struct DatabaseService {
        connection_string: String,
    }

    impl Service for DatabaseService {
        fn name(&self) -> &str {
            "DatabaseService"
        }

        fn execute(&self) -> String {
            format!("Connected to: {}", self.connection_string)
        }
    }

    #[derive(Debug)]
    struct CacheService {
        max_size: usize,
    }

    impl Service for CacheService {
        fn name(&self) -> &str {
            "CacheService"
        }

        fn execute(&self) -> String {
            format!("Cache with max size: {}", self.max_size)
        }
    }

    #[test]
    fn test_instance_new_creates_instance() {
        let data = Shared::new(TestData {
            id: 1,
            name: "test".to_string(),
        });

        let instance = Instance::new(data);

        assert_eq!(instance.get().id, 1);
        assert_eq!(instance.get().name, "test");
    }

    #[test]
    fn test_instance_new_with_primitive() {
        let value = Shared::new(42);
        let instance = Instance::new(value);

        assert_eq!(*instance.get(), 42);
    }

    #[test]
    fn test_instance_new_with_string() {
        let text = Shared::new(String::from("Hello, world!"));
        let instance = Instance::new(text);

        assert_eq!(instance.get().as_str(), "Hello, world!");
    }

    #[test]
    fn test_instance_new_with_vec() {
        let numbers = Shared::new(vec![1, 2, 3, 4, 5]);
        let instance = Instance::new(numbers);

        assert_eq!(instance.get().len(), 5);
        assert_eq!(instance.get()[2], 3);
    }

    #[test]
    fn test_instance_new_with_complex_struct() {
        #[derive(Debug, PartialEq)]
        struct ComplexData {
            values: Vec<i32>,
            metadata: std::collections::HashMap<String, String>,
        }

        let mut map = std::collections::HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());

        let complex = Shared::new(ComplexData {
            values: vec![10, 20, 30],
            metadata: map,
        });

        let instance = Instance::new(complex);

        assert_eq!(instance.get().values.len(), 3);
        assert_eq!(instance.get().metadata.get("key1").unwrap(), "value1");
    }

    #[test]
    fn test_get_returns_reference() {
        let data = Shared::new(TestData {
            id: 42,
            name: "reference-test".to_string(),
        });

        let instance = Instance::new(data);
        let reference = instance.get();

        assert_eq!(reference.id, 42);
        assert_eq!(reference.name, "reference-test");
    }

    #[test]
    fn test_get_multiple_times_returns_same_data() {
        let data = Shared::new(Counter { value: 100 });
        let instance = Instance::new(data);

        let ref1 = instance.get();
        let ref2 = instance.get();

        // Both references point to the same data
        assert_eq!(ref1.value, ref2.value);
        assert_eq!(ref1.value, 100);
    }

    #[test]
    fn test_get_allows_field_access() {
        let data = Shared::new(TestData {
            id: 5,
            name: "field-test".to_string(),
        });

        let instance = Instance::new(data);

        // Direct field access
        assert_eq!(instance.get().id, 5);
        assert_eq!(instance.get().name, "field-test");
    }

    #[test]
    fn test_get_allows_method_calls() {
        #[derive(Debug)]
        struct Calculator {
            base: i32,
        }

        impl Calculator {
            fn add(&self, x: i32) -> i32 {
                self.base + x
            }

            fn multiply(&self, x: i32) -> i32 {
                self.base * x
            }
        }

        let calc = Shared::new(Calculator { base: 10 });
        let instance = Instance::new(calc);

        assert_eq!(instance.get().add(5), 15);
        assert_eq!(instance.get().multiply(3), 30);
    }

    #[test]
    fn test_get_with_nested_access() {
        #[derive(Debug)]
        struct Inner {
            value: String,
        }

        #[derive(Debug)]
        struct Outer {
            inner: Inner,
            count: usize,
        }

        let outer = Shared::new(Outer {
            inner: Inner {
                value: "nested".to_string(),
            },
            count: 5,
        });

        let instance = Instance::new(outer);

        assert_eq!(instance.get().inner.value, "nested");
        assert_eq!(instance.get().count, 5);
    }

    #[test]
    fn test_value_returns_cloned_shared() {
        let data = Shared::new(TestData {
            id: 99,
            name: "clone-test".to_string(),
        });

        let instance = Instance::new(data);
        let shared1 = instance.value();
        let shared2 = instance.value();

        // Both point to the same allocation
        assert!(Shared::ptr_eq(&shared1, &shared2));
    }

    #[test]
    fn test_value_increments_reference_count() {
        let data = Shared::new(TestData {
            id: 1,
            name: "refcount".to_string(),
        });

        let instance = Instance::new(data.clone());

        // Initial count: 2 (data + instance)
        #[cfg(feature = "thread-safe")]
        let initial_count = std::sync::Arc::strong_count(&data);
        #[cfg(not(feature = "thread-safe"))]
        let initial_count = std::rc::Rc::strong_count(&data);

        let _shared1 = instance.value();

        #[cfg(feature = "thread-safe")]
        let after_one = std::sync::Arc::strong_count(&data);
        #[cfg(not(feature = "thread-safe"))]
        let after_one = std::rc::Rc::strong_count(&data);

        assert_eq!(after_one, initial_count + 1);

        let _shared2 = instance.value();

        #[cfg(feature = "thread-safe")]
        let after_two = std::sync::Arc::strong_count(&data);
        #[cfg(not(feature = "thread-safe"))]
        let after_two = std::rc::Rc::strong_count(&data);

        assert_eq!(after_two, initial_count + 2);
    }

    #[test]
    fn test_value_can_be_stored() {
        let data = Shared::new(TestData {
            id: 10,
            name: "storage-test".to_string(),
        });

        let instance = Instance::new(data);

        // Store in a vector
        let mut storage: Vec<Shared<TestData>> = Vec::new();
        storage.push(instance.value());
        storage.push(instance.value());
        storage.push(instance.value());

        // All stored references point to the same data
        assert!(Shared::ptr_eq(&storage[0], &storage[1]));
        assert!(Shared::ptr_eq(&storage[1], &storage[2]));

        // Data is accessible through stored references
        assert_eq!(storage[0].id, 10);
        assert_eq!(storage[2].name, "storage-test");
    }

    #[test]
    fn test_value_enables_sharing_across_components() {
        struct ServiceA {
            data: Shared<TestData>,
        }

        struct ServiceB {
            data: Shared<TestData>,
        }

        let data = Shared::new(TestData {
            id: 50,
            name: "shared".to_string(),
        });

        let instance = Instance::new(data);

        let service_a = ServiceA {
            data: instance.value(),
        };

        let service_b = ServiceB {
            data: instance.value(),
        };

        // Both services share the same data
        assert!(Shared::ptr_eq(&service_a.data, &service_b.data));
        assert_eq!(service_a.data.id, 50);
        assert_eq!(service_b.data.id, 50);
    }

    #[test]
    fn test_instance_with_trait_object() {
        let service: Shared<dyn Service> = Shared::new(DatabaseService {
            connection_string: "postgresql://localhost".to_string(),
        });

        let instance = Instance::<dyn Service>::new(service);

        assert_eq!(instance.get().name(), "DatabaseService");
        assert!(instance.get().execute().contains("postgresql"));
    }

    #[test]
    fn test_instance_trait_object_polymorphism() {
        let db_service: Shared<dyn Service> = Shared::new(DatabaseService {
            connection_string: "postgresql://localhost".to_string(),
        });

        let cache_service: Shared<dyn Service> = Shared::new(CacheService { max_size: 1000 });

        let instance1 = Instance::<dyn Service>::new(db_service);
        let instance2 = Instance::<dyn Service>::new(cache_service);

        assert_eq!(instance1.get().name(), "DatabaseService");
        assert_eq!(instance2.get().name(), "CacheService");
    }

    #[test]
    fn test_trait_object_value_cloning() {
        let service: Shared<dyn Service> = Shared::new(CacheService { max_size: 500 });

        let instance = Instance::<dyn Service>::new(service);

        let shared1 = instance.value();
        let shared2 = instance.value();

        assert!(Shared::ptr_eq(&shared1, &shared2));
        assert_eq!(shared1.name(), "CacheService");
    }

    #[test]
    fn test_instance_with_empty_struct() {
        #[derive(Debug, PartialEq)]
        struct Empty;

        let empty = Shared::new(Empty);
        let instance = Instance::new(empty);

        assert_eq!(*instance.get(), Empty);
    }

    #[test]
    fn test_instance_with_large_struct() {
        #[derive(Debug)]
        struct LargeStruct {
            data: [u8; 1024],
        }

        let large = Shared::new(LargeStruct { data: [42; 1024] });

        let instance = Instance::new(large);

        assert_eq!(instance.get().data[0], 42);
        assert_eq!(instance.get().data[1023], 42);
    }

    #[test]
    fn test_multiple_instances_same_shared() {
        let data = Shared::new(TestData {
            id: 777,
            name: "multi-instance".to_string(),
        });

        let instance1 = Instance::new(data.clone());
        let instance2 = Instance::new(data.clone());
        let instance3 = Instance::new(data.clone());

        // All instances point to the same allocation
        assert!(Shared::ptr_eq(&instance1.value(), &instance2.value()));
        assert!(Shared::ptr_eq(&instance2.value(), &instance3.value()));

        // Data is consistent across all instances
        assert_eq!(instance1.get().id, 777);
        assert_eq!(instance2.get().id, 777);
        assert_eq!(instance3.get().id, 777);
    }

    #[test]
    fn test_instance_outlives_original_shared() {
        let instance = {
            let data = Shared::new(TestData {
                id: 888,
                name: "lifetime-test".to_string(),
            });
            Instance::new(data)
        }; // data is dropped here

        // Instance still holds valid reference
        assert_eq!(instance.get().id, 888);
        assert_eq!(instance.get().name, "lifetime-test");
    }

    #[test]
    fn test_nested_instances() {
        #[derive(Debug)]
        struct Inner {
            value: u32,
        }

        #[derive(Debug)]
        struct Outer {
            inner_instance: Instance<Inner>,
        }

        let inner = Instance::new(Shared::new(Inner { value: 42 }));
        let outer = Instance::new(Shared::new(Outer {
            inner_instance: inner,
        }));

        assert_eq!(outer.get().inner_instance.get().value, 42);
    }

    #[cfg(feature = "debug")]
    #[test]
    fn test_instance_debug_format() {
        let data = Shared::new(TestData {
            id: 1,
            name: "debug-test".to_string(),
        });

        let instance = Instance::new(data);
        let debug_str = format!("{:?}", instance);

        // Should contain Instance in the debug output
        assert!(debug_str.contains("Instance"));
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn test_instance_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        // Instance should be Send + Sync when T is Send + Sync
        assert_send_sync::<Instance<TestData>>();
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn test_instance_can_be_shared_across_threads() {
        use std::sync::Arc;
        use std::thread;

        let instance = Arc::new(Instance::new(Shared::new(TestData {
            id: 123,
            name: "thread-test".to_string(),
        })));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let instance_clone = Arc::clone(&instance);
                thread::spawn(move || instance_clone.get().id)
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert_eq!(result, 123);
        }
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn test_instance_value_can_be_sent_to_thread() {
        use std::thread;

        let instance = Instance::new(Shared::new(TestData {
            id: 456,
            name: "send-test".to_string(),
        }));

        let shared = instance.value();

        let handle = thread::spawn(move || shared.id);

        let result = handle.join().unwrap();
        assert_eq!(result, 456);
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn test_multiple_threads_accessing_same_instance() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::thread;

        #[derive(Debug)]
        struct SharedCounter {
            value: AtomicU32,
        }

        let instance = Arc::new(Instance::new(Shared::new(SharedCounter {
            value: AtomicU32::new(0),
        })));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let instance_clone = Arc::clone(&instance);
                thread::spawn(move || {
                    for _ in 0..100 {
                        instance_clone.get().value.fetch_add(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let final_value = instance.get().value.load(Ordering::SeqCst);
        assert_eq!(final_value, 1000); // 10 threads * 100 increments
    }

    #[test]
    fn test_realistic_dependency_injection_scenario() {
        #[derive(Debug)]
        struct Config {
            database_url: String,
            cache_ttl: u64,
        }

        #[derive(Debug)]
        struct Application {
            config: Shared<Config>,
        }

        impl Application {
            fn new(config: Shared<Config>) -> Self {
                Self { config }
            }

            fn get_database_url(&self) -> &str {
                &self.config.database_url
            }
        }

        // Simulate DI container resolving a config
        let config_instance = Instance::new(Shared::new(Config {
            database_url: "postgresql://prod".to_string(),
            cache_ttl: 3600,
        }));

        // Application uses the resolved config
        let app = Application::new(config_instance.value());

        assert_eq!(app.get_database_url(), "postgresql://prod");
        assert_eq!(app.config.cache_ttl, 3600);
    }

    #[test]
    fn test_instance_in_collection() {
        let instances: Vec<Instance<TestData>> = vec![
            Instance::new(Shared::new(TestData {
                id: 1,
                name: "one".to_string(),
            })),
            Instance::new(Shared::new(TestData {
                id: 2,
                name: "two".to_string(),
            })),
            Instance::new(Shared::new(TestData {
                id: 3,
                name: "three".to_string(),
            })),
        ];

        assert_eq!(instances.len(), 3);
        assert_eq!(instances[0].get().id, 1);
        assert_eq!(instances[1].get().name, "two");
        assert_eq!(instances[2].get().id, 3);
    }

    #[test]
    fn test_instance_with_option() {
        let some_instance = Some(Instance::new(Shared::new(TestData {
            id: 100,
            name: "optional".to_string(),
        })));

        let none_instance: Option<Instance<TestData>> = None;

        assert!(some_instance.is_some());
        assert!(none_instance.is_none());

        if let Some(instance) = some_instance {
            assert_eq!(instance.get().id, 100);
        }
    }
}
