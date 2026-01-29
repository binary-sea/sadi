use crate::injector::Injector;

#[cfg(feature = "debug")]
pub trait Module: std::fmt::Debug {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![]
    }

    fn providers(&self, injector: &Injector);
}

#[cfg(not(feature = "debug"))]
pub trait Module {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![]
    }

    fn providers(&self, injector: &Injector);
}
