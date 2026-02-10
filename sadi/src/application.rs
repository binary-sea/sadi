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

use crate::injector::Injector;
use crate::module::Module;
use crate::runtime::Shared;

#[cfg(feature = "tracing")]
use tracing::{debug, info};

/// The main application container for dependency injection.
///
/// `Application` manages the lifecycle of modules and provides access to the root
/// dependency injector. It handles the bootstrap process, which recursively loads
/// all modules and their imports, creating a hierarchical injector structure.
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
    root: Option<Box<dyn Module>>,
    injector: Shared<Injector>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application")
            .field("injector", &"...")
            .field("root", &"<dyn Module>")
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
            injector: Shared::new(Injector::root()),
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

        #[cfg(feature = "tracing")]
        info!("Starting application bootstrap process");

        Self::load_module(self.injector.clone(), root);

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
    fn load_module(parent: Shared<Injector>, module: Box<dyn Module>) {
        #[cfg(feature = "tracing")]
        debug!("Loading module into injector hierarchy");

        let module_injector = Shared::new(Injector::child(parent.clone()));

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

            Self::load_module(module_injector.clone(), import);
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
    use std::cell::RefCell;
    use std::rc::Rc;

    struct EmptyModule;

    impl Module for EmptyModule {
        fn providers(&self, _injector: &Injector) {}
    }

    struct CountingModule {
        counter: Rc<RefCell<usize>>,
    }

    impl Module for CountingModule {
        fn providers(&self, _injector: &Injector) {
            *self.counter.borrow_mut() += 1;
        }
    }

    struct ModuleWithImports {
        counter: Rc<RefCell<usize>>,
    }

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
        let counter = Rc::new(RefCell::new(0));
        let module = CountingModule {
            counter: counter.clone(),
        };

        let mut app = Application::new(module);
        assert_eq!(*counter.borrow(), 0);

        app.bootstrap();
        assert_eq!(
            *counter.borrow(),
            1,
            "Module providers should be called during bootstrap"
        );
    }

    #[test]
    fn test_bootstrap_loads_imports_first() {
        let counter = Rc::new(RefCell::new(0));
        let module = ModuleWithImports {
            counter: counter.clone(),
        };

        let mut app = Application::new(module);
        app.bootstrap();

        // 2 imports + 1 root module = 3 calls
        assert_eq!(*counter.borrow(), 3, "All modules should be loaded");
    }

    #[test]
    fn test_application_can_be_created_with_different_modules() {
        let _app1 = Application::new(EmptyModule);
        let _app2 = Application::new(CountingModule {
            counter: Rc::new(RefCell::new(0)),
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

    struct NestedImportModule {
        counter: Rc<RefCell<usize>>,
        depth: usize,
    }

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

    #[test]
    fn test_deeply_nested_modules() {
        let counter = Rc::new(RefCell::new(0));
        let module = NestedImportModule {
            counter: counter.clone(),
            depth: 5,
        };

        let mut app = Application::new(module);
        app.bootstrap();

        // depth 5, 4, 3, 2, 1, 0 = 6 modules total
        assert_eq!(*counter.borrow(), 6, "All nested modules should be loaded");
    }
}
