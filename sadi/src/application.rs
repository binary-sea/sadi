//! Application container for bootstrapping and managing the dependency injection system.
//!
//! This module provides the [`Application`] struct, which serves as the main entry point
//! for configuring and initializing a dependency injection container with a modular structure.
//!
//! # Overview
//!
//! The `Application` manages:
//! - Root module registration
//! - Bootstrap process for loading modules and their dependencies
//! - Access to the root injector
//! - Hierarchical module loading with proper isolation
//!
//! # Thread Safety
//!
//! When the `thread-safe` feature is enabled, the [`Application`] requires the root module
//! to implement `Send + Sync`, allowing the application to be safely shared across threads.
//!
//! # Examples
//!
//! ```
//! use sadi::application::Application;
//! use sadi::module::Module;
//! use sadi::injector::Injector;
//!
//! struct AppModule;
//!
//! impl Module for AppModule {
//!     fn providers(&self, injector: &Injector) {
//!         // Register providers
//!     }
//! }
//!
//! let mut app = Application::new(AppModule);
//! app.bootstrap();
//!
//! let injector = app.injector();
//! // Use injector to resolve dependencies
//! ```

use std::any::TypeId;
use std::collections::HashMap;

use crate::error::Error;
use crate::injector::Injector;
use crate::module::Module;
use crate::module_instance::ModuleInstance;
use crate::runtime::Shared;

#[cfg(feature = "tracing")]
use tracing::{debug, info};

/// The main application container for dependency injection.
///
/// `Application` manages the lifecycle of modules and provides access to the root
/// dependency injector. It handles the bootstrap process, which recursively loads
/// all modules and their imports, creating a hierarchical injector structure.
///
/// # Thread Safety
///
/// With the `thread-safe` feature enabled, the application requires modules to implement
/// `Send + Sync` to ensure they can be safely shared across threads. Without this feature,
/// modules have no additional thread-safety requirements.
///
/// # Lifecycle
///
/// 1. **Creation**: Create an application with a root module using [`new()`](Application::new)
/// 2. **Bootstrap**: Call [`bootstrap()`](Application::bootstrap) to load all modules
/// 3. **Usage**: Access the injector via [`injector()`](Application::injector) to resolve dependencies
///
/// # Examples
///
/// ```
/// use sadi::application::Application;
/// use sadi::module::Module;
/// use sadi::injector::Injector;
///
/// struct MyAppModule;
///
/// impl Module for MyAppModule {
///     fn providers(&self, injector: &Injector) {
///         // Configure your providers
///     }
/// }
///
/// let mut app = Application::new(MyAppModule);
/// assert!(!app.is_bootstrapped());
///
/// app.bootstrap();
/// assert!(app.is_bootstrapped());
///
/// let injector = app.injector();
/// // Use injector to get services
/// ```
pub struct Application {
    #[cfg(not(feature = "thread-safe"))]
    root: Option<Box<dyn Module>>,
    #[cfg(feature = "thread-safe")]
    root: Option<Box<dyn Module + Send + Sync>>,
    root_module_type_id: Option<TypeId>,
    injector: Shared<Injector>,
    modules: HashMap<TypeId, Vec<ModuleInstance>>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application")
            .field("injector", &"...")
            .field("root", &"<dyn Module>")
            .field("root_module_type_id", &self.root_module_type_id)
            .field(
                "modules_count",
                &self.modules.values().map(Vec::len).sum::<usize>(),
            )
            .finish()
    }
}

impl Application {
    /// Creates a new application with the given root module.
    ///
    /// The application is created in an un-bootstrapped state. You must call
    /// [`bootstrap()`](Application::bootstrap) to load the module and its dependencies.
    ///
    /// # Parameters
    ///
    /// - `root`: The root module that defines the application's dependency graph
    ///
    /// # Returns
    ///
    /// A new `Application` instance ready to be bootstrapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::application::Application;
    /// use sadi::module::Module;
    /// use sadi::injector::Injector;
    ///
    /// struct RootModule;
    ///
    /// impl Module for RootModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let app = Application::new(RootModule);
    /// assert!(!app.is_bootstrapped());
    /// ```
    pub fn new(root: impl Module + 'static) -> Self {
        #[cfg(feature = "tracing")]
        info!("Creating new Application instance with root module");

        Self {
            root: Some(Box::new(root)),
            root_module_type_id: None,
            injector: Shared::new(Injector::root()),
            modules: HashMap::new(),
        }
    }

    /// Bootstraps the application by loading the root module and all its imports.
    ///
    /// This method recursively processes the module hierarchy:
    /// 1. Creates child injectors for each module
    /// 2. Loads all imported modules first
    /// 3. Registers the module's own providers
    ///
    /// # Panics
    ///
    /// Panics if called more than once on the same application instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::application::Application;
    /// use sadi::module::Module;
    /// use sadi::injector::Injector;
    ///
    /// struct AppModule;
    ///
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {
    ///         // Register providers
    ///     }
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// app.bootstrap();
    /// assert!(app.is_bootstrapped());
    /// ```
    ///
    /// # Panics Example
    ///
    /// ```should_panic
    /// use sadi::application::Application;
    /// use sadi::module::Module;
    /// use sadi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// app.bootstrap();
    /// app.bootstrap(); // Panics: Application already bootstrapped
    /// ```
    pub fn bootstrap(&mut self) {
        let root = self.root.take().expect("Application already bootstrapped");
        self.root_module_type_id = Some(root.type_id());

        #[cfg(feature = "tracing")]
        info!("Starting application bootstrap process");

        self.load_module(self.injector.clone(), root, None);

        #[cfg(feature = "tracing")]
        info!("Application bootstrap completed successfully");
    }

    /// Returns a shared reference to the root injector.
    ///
    /// The injector can be used to resolve dependencies after the application
    /// has been bootstrapped. The returned reference can be cloned to share
    /// access to the injector.
    ///
    /// # Returns
    ///
    /// A shared reference to the root [`Injector`].
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::application::Application;
    /// use sadi::module::Module;
    /// use sadi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// app.bootstrap();
    ///
    /// let injector = app.injector();
    /// let another_ref = app.injector();
    /// // Both references point to the same injector
    /// ```
    pub fn injector(&self) -> Shared<Injector> {
        #[cfg(feature = "tracing")]
        debug!("Accessing root injector");

        #[cfg(feature = "tracing")]
        {
            if self.is_bootstrapped() {
                debug!("Injector is available and application is bootstrapped");
            } else {
                debug!("Injector is available but application is not bootstrapped yet");
            }
        }

        self.injector.clone()
    }

    /// Returns the injector associated with a bootstrapped module type.
    ///
    /// This lookup is keyed by module concrete type (`T`). It returns `None` when:
    /// - the application was not bootstrapped yet, or
    /// - no module of type `T` exists in the loaded graph.
    ///
    /// # Note about duplicate module types
    ///
    /// If multiple instances of the same module type are imported, the first loaded
    /// instance is kept in the registry for lookup.
    pub fn module<T>(&self) -> Option<&Injector>
    where
        T: Module + 'static,
    {
        self.module_global::<T>().or_else(|| {
            self.modules
                .get(&TypeId::of::<T>())
                .and_then(|modules| modules.first())
                .map(|module| module.injector())
        })
    }

    /// Returns a unique module injector for the given module type.
    ///
    /// Fails when no module is found or when multiple module instances exist.
    /// In case of duplicates, prefer [`module_global`](Application::module_global)
    /// or [`modules_in`](Application::modules_in) for explicit resolution.
    pub fn try_module_unique<T>(&self) -> Result<&Injector, Error>
    where
        T: Module + 'static,
    {
        let modules = self
            .modules
            .get(&TypeId::of::<T>())
            .ok_or_else(|| Error::module_not_found(std::any::type_name::<T>()))?;

        if modules.is_empty() {
            return Err(Error::module_not_found(std::any::type_name::<T>()));
        }

        if modules.len() > 1 {
            return Err(Error::ambiguous_module(
                std::any::type_name::<T>(),
                modules.len(),
            ));
        }

        Ok(modules[0].injector())
    }

    pub fn module_unique<T>(&self) -> &Injector
    where
        T: Module + 'static,
    {
        self.try_module_unique::<T>().unwrap()
    }

    /// Returns all module injectors for a given module type.
    pub fn modules<T>(&self) -> Vec<&Injector>
    where
        T: Module + 'static,
    {
        self.modules
            .get(&TypeId::of::<T>())
            .map(|modules| modules.iter().map(|module| module.injector()).collect())
            .unwrap_or_default()
    }

    /// Returns the global (application-level) module injector for the given type.
    ///
    /// A global module is one imported directly by the root module.
    pub fn module_global<T>(&self) -> Option<&Injector>
    where
        T: Module + 'static,
    {
        let root_type_id = self.root_module_type_id?;
        let target_type_id = TypeId::of::<T>();

        self.modules
            .get(&target_type_id)
            .and_then(|modules| {
                modules
                    .iter()
                    .find(|module| module.parent_type_id() == Some(root_type_id))
                    .or_else(|| {
                        if target_type_id == root_type_id {
                            modules
                                .iter()
                                .find(|module| module.parent_type_id().is_none())
                        } else {
                            None
                        }
                    })
            })
            .map(|module| module.injector())
    }

    /// Returns all module injectors of type `T` that were loaded under parent module `P`.
    pub fn modules_in<P, T>(&self) -> Vec<&Injector>
    where
        P: Module + 'static,
        T: Module + 'static,
    {
        let parent_type_id = TypeId::of::<P>();

        self.modules
            .get(&TypeId::of::<T>())
            .map(|modules| {
                modules
                    .iter()
                    .filter(|module| module.parent_type_id() == Some(parent_type_id))
                    .map(|module| module.injector())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Checks whether the application has been bootstrapped.
    ///
    /// Returns `true` if [`bootstrap()`](Application::bootstrap) has been called,
    /// `false` otherwise.
    ///
    /// # Returns
    ///
    /// - `true` if the application is bootstrapped
    /// - `false` if the application has not been bootstrapped yet
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::application::Application;
    /// use sadi::module::Module;
    /// use sadi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// assert!(!app.is_bootstrapped());
    ///
    /// app.bootstrap();
    /// assert!(app.is_bootstrapped());
    /// ```
    pub fn is_bootstrapped(&self) -> bool {
        let bootstrapped = self.root.is_none();

        #[cfg(feature = "tracing")]
        debug!("Checking application bootstrap state: {}", bootstrapped);

        bootstrapped
    }

    /// Recursively loads a module and its imports into the injector hierarchy.
    ///
    /// Creates a child injector for the module, loads all imported modules first,
    /// then registers the module's own providers. This ensures proper dependency
    /// resolution order.
    ///
    /// # Parameters
    ///
    /// - `parent`: The parent injector to create a child from
    /// - `module`: The module to load
    fn load_module(
        &mut self,
        parent: Shared<Injector>,
        module: Box<dyn Module>,
        parent_module_type_id: Option<TypeId>,
    ) {
        #[cfg(feature = "tracing")]
        debug!("Loading module into injector hierarchy");

        println!(
            "Loading module: {:?} with id: {:?}",
            module.type_name(),
            module.type_id()
        );

        let module_injector = Shared::new(Injector::child(parent.clone()));

        let module_type_id = module.type_id();

        self.modules
            .entry(module_type_id)
            .or_default()
            .push(ModuleInstance::new(
                module.as_ref(),
                module_injector.as_ref().clone(),
                parent_module_type_id,
            ));

        #[cfg(feature = "tracing")]
        debug!("Created child injector for module");

        let imports = module.imports();
        #[cfg(feature = "tracing")]
        if !imports.is_empty() {
            debug!("Module has {} imports, loading them first", imports.len());
        }

        #[allow(unused_variables)]
        for (index, import) in imports.into_iter().enumerate() {
            #[cfg(feature = "tracing")]
            debug!("Loading import {}", index + 1);

            self.load_module(module_injector.clone(), import, Some(module_type_id));
        }

        #[cfg(feature = "tracing")]
        debug!("Registering module providers");

        module.providers(&module_injector);

        #[cfg(feature = "tracing")]
        debug!("Module loaded successfully");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "thread-safe"))]
    use std::cell::RefCell;
    #[cfg(not(feature = "thread-safe"))]
    use std::rc::Rc;

    #[cfg(feature = "thread-safe")]
    use std::sync::{Arc, Mutex};

    struct EmptyModule;

    impl Module for EmptyModule {
        fn providers(&self, _injector: &Injector) {}
    }

    struct ImportedLookupModule;

    impl Module for ImportedLookupModule {
        fn providers(&self, _injector: &Injector) {}
    }

    struct RootLookupModule;

    impl Module for RootLookupModule {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![Box::new(ImportedLookupModule)]
        }

        fn providers(&self, _injector: &Injector) {}
    }

    struct ModuleA;
    struct ModuleB;
    struct ModuleC;

    impl Module for ModuleA {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![Box::new(ModuleB)]
        }

        fn providers(&self, _injector: &Injector) {}
    }

    impl Module for ModuleB {
        fn providers(&self, _injector: &Injector) {}
    }

    impl Module for ModuleC {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![Box::new(ModuleB)]
        }

        fn providers(&self, _injector: &Injector) {}
    }

    struct RootWithRepeatedB;

    impl Module for RootWithRepeatedB {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![Box::new(ModuleA), Box::new(ModuleB), Box::new(ModuleC)]
        }

        fn providers(&self, _injector: &Injector) {}
    }

    // CountingModule with conditional thread safety
    #[cfg(not(feature = "thread-safe"))]
    struct CountingModule {
        counter: Rc<RefCell<usize>>,
    }

    #[cfg(not(feature = "thread-safe"))]
    impl Module for CountingModule {
        fn providers(&self, _injector: &Injector) {
            *self.counter.borrow_mut() += 1;
        }
    }

    #[cfg(feature = "thread-safe")]
    struct CountingModule {
        counter: Arc<Mutex<usize>>,
    }

    #[cfg(feature = "thread-safe")]
    impl Module for CountingModule {
        fn providers(&self, _injector: &Injector) {
            *self.counter.lock().unwrap() += 1;
        }
    }

    // ModuleWithImports with conditional thread safety
    #[cfg(not(feature = "thread-safe"))]
    struct ModuleWithImports {
        counter: Rc<RefCell<usize>>,
    }

    #[cfg(not(feature = "thread-safe"))]
    impl Module for ModuleWithImports {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
            ]
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.borrow_mut() += 1;
        }
    }

    #[cfg(feature = "thread-safe")]
    struct ModuleWithImports {
        counter: Arc<Mutex<usize>>,
    }

    #[cfg(feature = "thread-safe")]
    impl Module for ModuleWithImports {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
            ]
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.lock().unwrap() += 1;
        }
    }

    #[test]
    fn test_new_creates_unbootstrapped_application() {
        let app = Application::new(EmptyModule);
        assert!(
            !app.is_bootstrapped(),
            "New application should not be bootstrapped"
        );
    }

    #[test]
    fn test_bootstrap_changes_state() {
        let mut app = Application::new(EmptyModule);
        assert!(!app.is_bootstrapped());

        app.bootstrap();
        assert!(
            app.is_bootstrapped(),
            "Application should be bootstrapped after bootstrap()"
        );
    }

    #[test]
    #[should_panic(expected = "Application already bootstrapped")]
    fn test_bootstrap_twice_panics() {
        let mut app = Application::new(EmptyModule);
        app.bootstrap();
        app.bootstrap(); // Should panic
    }

    #[test]
    fn test_injector_returns_shared_reference() {
        let mut app = Application::new(EmptyModule);
        app.bootstrap();

        let injector1 = app.injector();
        let _injector2 = app.injector();

        // Both should reference the same underlying injector
        #[cfg(feature = "thread-safe")]
        assert_eq!(std::sync::Arc::strong_count(&injector1), 3); // app + injector1 + injector2

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(std::rc::Rc::strong_count(&injector1), 3); // app + injector1 + injector2
    }

    #[test]
    fn test_bootstrap_calls_module_providers() {
        #[cfg(not(feature = "thread-safe"))]
        let counter = Rc::new(RefCell::new(0));
        #[cfg(feature = "thread-safe")]
        let counter = Arc::new(Mutex::new(0));

        let module = CountingModule {
            counter: counter.clone(),
        };

        let mut app = Application::new(module);

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(*counter.borrow(), 0);
        #[cfg(feature = "thread-safe")]
        assert_eq!(*counter.lock().unwrap(), 0);

        app.bootstrap();

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(
            *counter.borrow(),
            1,
            "Module providers should be called during bootstrap"
        );
        #[cfg(feature = "thread-safe")]
        assert_eq!(
            *counter.lock().unwrap(),
            1,
            "Module providers should be called during bootstrap"
        );
    }

    #[test]
    fn test_bootstrap_loads_imports_first() {
        #[cfg(not(feature = "thread-safe"))]
        let counter = Rc::new(RefCell::new(0));
        #[cfg(feature = "thread-safe")]
        let counter = Arc::new(Mutex::new(0));

        let module = ModuleWithImports {
            counter: counter.clone(),
        };

        let mut app = Application::new(module);
        app.bootstrap();

        // 2 imports + 1 root module = 3 calls
        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(*counter.borrow(), 3, "All modules should be loaded");
        #[cfg(feature = "thread-safe")]
        assert_eq!(*counter.lock().unwrap(), 3, "All modules should be loaded");
    }

    #[test]
    fn test_application_can_be_created_with_different_modules() {
        let _app1 = Application::new(EmptyModule);

        #[cfg(not(feature = "thread-safe"))]
        let _app2 = Application::new(CountingModule {
            counter: Rc::new(RefCell::new(0)),
        });
        #[cfg(feature = "thread-safe")]
        let _app2 = Application::new(CountingModule {
            counter: Arc::new(Mutex::new(0)),
        });

        // Should compile and work with different module types
    }

    #[test]
    fn test_injector_accessible_before_bootstrap() {
        let app = Application::new(EmptyModule);
        let _injector = app.injector();
        // Should not panic - injector is available even before bootstrap
    }

    #[test]
    fn test_multiple_injector_clones() {
        let mut app = Application::new(EmptyModule);
        app.bootstrap();

        let injectors: Vec<_> = (0..5).map(|_| app.injector()).collect();
        assert_eq!(injectors.len(), 5);

        #[cfg(feature = "thread-safe")]
        assert_eq!(std::sync::Arc::strong_count(&injectors[0]), 6); // app + 5 in vec

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(std::rc::Rc::strong_count(&injectors[0]), 6); // app + 5 in vec
    }

    #[test]
    fn test_module_lookup_returns_none_before_bootstrap() {
        let app = Application::new(RootLookupModule);
        assert!(app.module::<RootLookupModule>().is_none());
    }

    #[test]
    fn test_module_lookup_finds_root_and_imported_modules() {
        let mut app = Application::new(RootLookupModule);
        app.bootstrap();

        assert!(app.module::<RootLookupModule>().is_some());
        assert!(app.module::<ImportedLookupModule>().is_some());
        assert!(app.module::<EmptyModule>().is_none());
    }

    #[test]
    fn test_module_lookup_supports_global_and_contextual_resolution() {
        let mut app = Application::new(RootWithRepeatedB);
        app.bootstrap();

        let all_b = app.modules::<ModuleB>();
        assert_eq!(all_b.len(), 3);

        assert!(app.module_global::<ModuleB>().is_some());
        assert_eq!(app.modules_in::<ModuleA, ModuleB>().len(), 1);
        assert_eq!(app.modules_in::<ModuleC, ModuleB>().len(), 1);
    }

    #[test]
    fn test_module_unique_returns_single_module() {
        let mut app = Application::new(RootLookupModule);
        app.bootstrap();

        let module = app.try_module_unique::<ImportedLookupModule>();
        assert!(module.is_ok());
    }

    #[test]
    fn test_module_unique_fails_for_ambiguous_module() {
        let mut app = Application::new(RootWithRepeatedB);
        app.bootstrap();

        let err = app.try_module_unique::<ModuleB>().unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::AmbiguousModule);
    }

    #[cfg(feature = "debug")]
    #[test]
    fn test_debug_implementation() {
        let app = Application::new(EmptyModule);
        let debug_str = format!("{:?}", app);
        assert!(
            debug_str.contains("Application"),
            "Debug output should contain 'Application'"
        );
    }

    // NestedImportModule with conditional thread safety
    #[cfg(not(feature = "thread-safe"))]
    struct NestedImportModule {
        counter: Rc<RefCell<usize>>,
        depth: usize,
    }

    #[cfg(not(feature = "thread-safe"))]
    impl Module for NestedImportModule {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            if self.depth > 0 {
                vec![Box::new(NestedImportModule {
                    counter: self.counter.clone(),
                    depth: self.depth - 1,
                })]
            } else {
                vec![]
            }
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.borrow_mut() += 1;
        }
    }

    #[cfg(feature = "thread-safe")]
    struct NestedImportModule {
        counter: Arc<Mutex<usize>>,
        depth: usize,
    }

    #[cfg(feature = "thread-safe")]
    impl Module for NestedImportModule {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            if self.depth > 0 {
                vec![Box::new(NestedImportModule {
                    counter: self.counter.clone(),
                    depth: self.depth - 1,
                })]
            } else {
                vec![]
            }
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.lock().unwrap() += 1;
        }
    }

    #[test]
    fn test_deeply_nested_modules() {
        #[cfg(not(feature = "thread-safe"))]
        let counter = Rc::new(RefCell::new(0));
        #[cfg(feature = "thread-safe")]
        let counter = Arc::new(Mutex::new(0));

        let module = NestedImportModule {
            counter: counter.clone(),
            depth: 5,
        };

        let mut app = Application::new(module);
        app.bootstrap();

        // depth 5, 4, 3, 2, 1, 0 = 6 modules total
        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(*counter.borrow(), 6, "All nested modules should be loaded");
        #[cfg(feature = "thread-safe")]
        assert_eq!(
            *counter.lock().unwrap(),
            6,
            "All nested modules should be loaded"
        );
    }
}
