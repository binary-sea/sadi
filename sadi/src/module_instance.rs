use std::any::TypeId;

use crate::injector::Injector;
use crate::module::Module;

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct ModuleInstance {
    type_name: String,
    type_id: TypeId,
    parent_type_id: Option<TypeId>,
    injector: Injector,
}

impl ModuleInstance {
    pub fn new(
        value: &(dyn Module + 'static),
        injector: Injector,
        parent_type_id: Option<TypeId>,
    ) -> Self {
        Self {
            type_name: value.type_name().to_string(),
            type_id: value.type_id(),
            parent_type_id,
            injector,
        }
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn parent_type_id(&self) -> Option<TypeId> {
        self.parent_type_id
    }

    pub fn injector(&self) -> &Injector {
        &self.injector
    }
}
