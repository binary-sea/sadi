/// Defines the lifecycle scope of a service in the dependency injection container.
///
/// # Variants
///
/// * `Root` - Root-level singleton. The instance is created once and shared
///   across the entire application during its complete lifecycle.
///
/// * `Module` - Module-level singleton. The instance is created once per module
///   and shared among all consumers within that specific module.
///
/// * `Transient` - Transient instance. A new instance is created each time
///   the service is requested from the dependency injection container.
///
/// # Examples
///
/// ```no_run
/// use sadi::Scope;
///
/// let root_scope = Scope::Root;
/// assert!(root_scope.is_singleton());
///
/// let transient_scope = Scope::Transient;
/// assert!(!transient_scope.is_singleton());
/// ```
#[derive(Clone, Copy)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum Scope {
    Root,
    Module,
    Transient,
}

impl Scope {
    /// Checks if this scope represents a singleton service.
    ///
    /// Returns `true` for `Root` and `Module` scopes, which create a single shared
    /// instance. Returns `false` for `Transient` scope, which creates a new
    /// instance on each request.
    ///
    /// # Returns
    ///
    /// * `true` - If the scope is `Root` or `Module` (singleton)
    /// * `false` - If the scope is `Transient` (not singleton)
    ///
    /// # Examples
    ///
    /// ```
    /// use sadi::Scope;
    ///
    /// assert!(Scope::Root.is_singleton());
    /// assert!(Scope::Module.is_singleton());
    /// assert!(!Scope::Transient.is_singleton());
    /// ```
    pub fn is_singleton(self) -> bool {
        matches!(self, Scope::Root | Scope::Module)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_is_singleton() {
        let scope = Scope::Root;
        assert!(scope.is_singleton(), "Scope::Root should be singleton");
    }

    #[test]
    fn test_module_is_singleton() {
        let scope = Scope::Module;
        assert!(scope.is_singleton(), "Scope::Module should be singleton");
    }

    #[test]
    fn test_transient_is_not_singleton() {
        let scope = Scope::Transient;
        assert!(
            !scope.is_singleton(),
            "Scope::Transient should not be singleton"
        );
    }

    #[test]
    fn test_scope_is_copy() {
        let scope1 = Scope::Root;
        let scope2 = scope1;
        // Both should be equal after copy
        assert!(scope1.is_singleton());
        assert!(scope2.is_singleton());
    }

    #[test]
    fn test_scope_is_clone() {
        let scope1 = Scope::Module;
        let scope2 = scope1.clone();
        assert!(scope1.is_singleton());
        assert!(scope2.is_singleton());
    }

    #[test]
    fn test_all_scopes_are_covered() {
        // Test that all enum variants have been considered
        let scopes = [Scope::Root, Scope::Module, Scope::Transient];

        let singleton_count = scopes.iter().filter(|s| s.is_singleton()).count();
        assert_eq!(
            singleton_count, 2,
            "There should be exactly 2 singleton scopes"
        );
    }
}
