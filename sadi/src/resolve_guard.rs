use std::cell::RefCell;

use crate::Error;

thread_local! {
    // Stack of type names being resolved in this thread.
    // Using String so we can build and report the chain.
    static RESOLVE_STACK: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Guard that pops the last pushed type name on Drop.
pub struct ResolveGuard {
    pub type_name: String,
}

impl ResolveGuard {
    /// Try to push a type_name onto the thread-local stack.
    /// Returns Err(Error::circular_dependency(..)) if the type is already on the stack.
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
