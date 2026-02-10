//! Module system for organizing dependency injection providers.
//!
//! This module defines the [`Module`] trait, which serves as the foundation for organizing
//! and configuring dependency injection in a modular, composable way.
//!
//! # Overview
//!
//! Modules allow you to:
//! - Group related providers together
//! - Import other modules to compose functionality
//! - Configure services within an injector
//!
//! # Examples
//!
//! ```
//! use sadi::module::Module;
//! use sadi::injector::Injector;
//!
//! struct DatabaseModule;
//!
//! impl Module for DatabaseModule {
//!     fn providers(&self, injector: &Injector) {
//!         // Register database-related providers
//!     }
//! }
//! ```

use crate::injector::Injector;

/// Trait for defining a module in the dependency injection system.
///
/// A module encapsulates a set of providers and can import other modules to build
/// a hierarchical dependency injection configuration. Modules are the primary way
/// to organize and structure your application's services.
///
/// # Required Methods
///
/// - [`providers`](Module::providers): Registers providers with the injector
///
/// # Optional Methods
///
/// - [`imports`](Module::imports): Returns other modules that this module depends on
///
/// # Examples
///
/// ## Basic Module
///
/// ```
/// use sadi::module::Module;
/// use sadi::injector::Injector;
///
/// struct LoggingModule;
///
/// impl Module for LoggingModule {
///     fn providers(&self, injector: &Injector) {
///         // Register logging providers
///     }
/// }
/// ```
///
/// ## Module with Imports
///
/// ```
/// use sadi::module::Module;
/// use sadi::injector::Injector;
///
/// struct DatabaseModule;
/// struct ConfigModule;
///
/// impl Module for DatabaseModule {
///     fn providers(&self, injector: &Injector) {
///         // Register database providers
///     }
/// }
///
/// impl Module for ConfigModule {
///     fn providers(&self, injector: &Injector) {
///         // Register config providers
///     }
/// }
///
/// struct AppModule;
///
/// impl Module for AppModule {
///     fn imports(&self) -> Vec<Box<dyn Module>> {
///         vec![
///             Box::new(DatabaseModule),
///             Box::new(ConfigModule),
///         ]
///     }
///
///     fn providers(&self, injector: &Injector) {
///         // Register app-level providers
///     }
/// }
/// ```
pub trait Module {
    /// Returns a list of modules that this module imports.
    ///
    /// Imported modules have their providers registered before this module's providers.
    /// This allows a module to build upon functionality provided by other modules.
    ///
    /// # Default Implementation
    ///
    /// By default, returns an empty vector (no imports).
    ///
    /// # Returns
    ///
    /// A vector of boxed `Module` trait objects representing the imported modules.
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::module::Module;
    /// use sadi::injector::Injector;
    ///
    /// struct CoreModule;
    /// impl Module for CoreModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// struct FeatureModule;
    /// impl Module for FeatureModule {
    ///     fn imports(&self) -> Vec<Box<dyn Module>> {
    ///         vec![Box::new(CoreModule)]
    ///     }
    ///
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    /// ```
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![]
    }

    /// Registers providers with the given injector.
    ///
    /// This method is called to configure the dependency injection container with
    /// the services that this module provides. Use the injector to register
    /// factories, values, and other providers.
    ///
    /// # Parameters
    ///
    /// - `injector`: The injector instance to register providers with
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::module::Module;
    /// use sadi::injector::Injector;
    ///
    /// struct MyModule;
    ///
    /// impl Module for MyModule {
    ///     fn providers(&self, injector: &Injector) {
    ///         // Register providers here
    ///         // injector.register<...>(...)
    ///     }
    /// }
    /// ```
    fn providers(&self, _injector: &Injector) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyModule;

    impl Module for EmptyModule {
        fn providers(&self, _injector: &Injector) {}
    }

    struct ModuleWithImports {
        import_count: usize,
    }

    impl Module for ModuleWithImports {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            (0..self.import_count)
                .map(|_| Box::new(EmptyModule) as Box<dyn Module>)
                .collect()
        }

        fn providers(&self, _injector: &Injector) {}
    }

    #[test]
    fn test_default_imports_returns_empty_vec() {
        let module = EmptyModule;
        let imports = module.imports();
        assert!(imports.is_empty(), "Default imports should be empty");
    }

    #[test]
    fn test_module_can_have_imports() {
        let module = ModuleWithImports { import_count: 3 };
        let imports = module.imports();
        assert_eq!(imports.len(), 3, "Should have 3 imports");
    }

    #[test]
    fn test_module_providers_can_be_called() {
        let module = EmptyModule;
        let injector = Injector::root();

        // Should not panic
        module.providers(&injector);
    }

    #[test]
    fn test_module_trait_object() {
        let module: Box<dyn Module> = Box::new(EmptyModule);
        let injector = Injector::root();

        // Test that trait object works correctly
        let imports = module.imports();
        assert!(imports.is_empty());

        module.providers(&injector);
    }

    #[test]
    fn test_multiple_modules() {
        let modules: Vec<Box<dyn Module>> = vec![
            Box::new(EmptyModule),
            Box::new(EmptyModule),
            Box::new(ModuleWithImports { import_count: 2 }),
        ];

        assert_eq!(modules.len(), 3, "Should have 3 modules");

        let injector = Injector::root();
        for module in modules {
            module.providers(&injector);
        }
    }

    #[test]
    fn test_nested_imports() {
        let module = ModuleWithImports { import_count: 2 };
        let imports = module.imports();

        // Each import should also be callable
        let injector = Injector::root();
        for import in imports {
            import.providers(&injector);
            assert!(
                import.imports().is_empty(),
                "Nested imports should be empty for EmptyModule"
            );
        }
    }

    struct CountingModule {
        call_count: std::cell::RefCell<usize>,
    }

    impl Module for CountingModule {
        fn providers(&self, _injector: &Injector) {
            *self.call_count.borrow_mut() += 1;
        }
    }

    #[test]
    fn test_providers_can_have_side_effects() {
        let module = CountingModule {
            call_count: std::cell::RefCell::new(0),
        };
        let injector = Injector::root();

        assert_eq!(*module.call_count.borrow(), 0);

        module.providers(&injector);
        assert_eq!(*module.call_count.borrow(), 1);

        module.providers(&injector);
        assert_eq!(*module.call_count.borrow(), 2);
    }
}
