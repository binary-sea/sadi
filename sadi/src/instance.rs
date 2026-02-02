use std::any::Any;

use crate::runtime::Shared;

pub struct Instance<T: ?Sized + 'static> {
    pub value: Shared<T>,
}

impl<T: ?Sized + 'static> Instance<T> {
    pub fn new(value: Shared<T>) -> Self {
        Self { value }
    }

    pub fn as_any(self) -> Shared<dyn Any + 'static> {
        Shared::new(self)
    }
}

impl<T: ?Sized + 'static> Clone for Instance<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}
