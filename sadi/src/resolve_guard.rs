use std::{any::TypeId, cell::RefCell};

use crate::error::{Error, ErrorKind};

thread_local! {
    static RESOLVE_STACK: RefCell<Vec<TypeId>> = RefCell::new(Vec::new());
}

pub struct ResolveGuard {
    type_id: TypeId,
}

impl ResolveGuard {
    pub fn push(type_id: TypeId) -> Result<Self, Error> {
        RESOLVE_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();

            if stack.contains(&type_id) {
                return Err(Error::new(
                    ErrorKind::CircularDependency,
                    format!(
                        "Circular dependency detected while resolving type_id: {:?}",
                        type_id
                    ),
                ));
            }

            stack.push(type_id);
            Ok(Self { type_id })
        })
    }
}

impl Drop for ResolveGuard {
    fn drop(&mut self) {
        RESOLVE_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            if let Some(last) = stack.pop() {
                if last != self.type_id {
                    panic!(
                        "ResolveGuard stack corrupted: expected to pop {:?} but popped {:?}",
                        self.type_id, last
                    );
                }
            } else {
                panic!("ResolveGuard stack corrupted: attempted to pop from an empty stack");
            }
        });
    }
}
