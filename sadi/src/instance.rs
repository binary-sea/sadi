use crate::Shared;

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Instance<T: ?Sized + 'static> {
    value: Shared<T>,
}


impl<T: ?Sized + 'static> Instance<T> {
    pub fn new(value: Shared<T>) -> Self {
        Self { value }
    }

    pub fn get(&self) -> &T {
        &*self.value
    }

    pub fn value(&self) -> Shared<T> {
        self.value.clone()
    }
}

