//! Thread-local stack guard for circular dependency detection in SaDi.
//!
//! This module provides [`ResolveGuard`], a utility for tracking the chain of type names
//! being resolved during dependency injection. It uses a thread-local stack to detect
//! and report circular dependencies, returning a detailed error chain if a cycle is found.
//!
//! # Example
//! ```
//! use sadi::{ErrorKind, ResolveGuard};
//!
//! // Push a type name onto the stack
//! let _g1 = ResolveGuard::push("A").unwrap();
//! // Pushing a different type is fine
//! let _g2 = ResolveGuard::push("B").unwrap();
//! // Pushing the same type again triggers a circular dependency error
//! let err = ResolveGuard::push("A").unwrap_err();
//! assert!(matches!(err.kind, ErrorKind::CircularDependency));
//! ```

use std::cell::RefCell;

use crate::Error;

thread_local! {
    // Stack of type names being resolved in this thread.
    // Using String so we can build and report the chain.
    static RESOLVE_STACK: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Guard that pops the last pushed type name from the thread-local stack on drop.
///
/// Used to track the current dependency resolution chain for circular detection.
#[derive(Debug)]
pub struct ResolveGuard {
    pub type_name: String,
}

impl ResolveGuard {
    /// Try to push a type name onto the thread-local stack.
    ///
    /// Returns `Err(Error::circular_dependency(..))` if the type is already on the stack.
    /// Otherwise, returns a guard that will pop the type on drop.
    pub fn push(type_name: &str) -> Result<Self, Error> {
        RESOLVE_STACK.with(|stack| {
            let mut v = stack.borrow_mut();
            if v.iter().any(|s| s == type_name) {
                // Build chain: existing stack + current type
                let mut chain = v.clone();
                chain.push(type_name.to_string());
                // Convert to Vec<&str> for Error::circular_dependency
                let refs: Vec<&str> = chain.iter().map(|s| s.as_str()).collect();
                return Err(Error::circular_dependency(&refs));
            }
            v.push(type_name.to_string());
            Ok(ResolveGuard {
                type_name: type_name.to_string(),
            })
        })
    }
}

impl Drop for ResolveGuard {
    fn drop(&mut self) {
        RESOLVE_STACK.with(|stack| {
            let mut v = stack.borrow_mut();
            v.pop();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;

    #[test]
    fn push_and_pop_stack() {
        // Push A, then B, then pop B, then pop A
        {
            let _g1 = ResolveGuard::push("A").unwrap();
            {
                let _g2 = ResolveGuard::push("B").unwrap();
                // B is on top
                let err = ResolveGuard::push("A").unwrap_err();
                assert!(matches!(err.kind, ErrorKind::CircularDependency));
            }
            // B popped, only A remains
            assert!(ResolveGuard::push("A").is_err());
        }
        // All popped, stack is empty, can push A again
        let _g = ResolveGuard::push("A").unwrap();
    }
}
