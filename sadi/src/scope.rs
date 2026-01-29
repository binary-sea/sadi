#[derive(Clone, Copy)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum Scope {
    Root,
    Module,
    Transient,
}

impl Scope {
    pub fn is_singleton(self) -> bool {
        matches!(self, Scope::Root | Scope::Module)
    }
}
