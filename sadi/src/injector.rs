use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::provider::Provider;
use crate::runtime::{Shared, Store};

pub struct Injector {
    inner: Shared<InjectorInner>,
}

struct InjectorInner {
    parent: Option<Shared<InjectorInner>>,
    providers: Store<HashMap<TypeId, Provider>>,
    instances: Store<HashMap<TypeId, Shared<dyn Any>>>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for InjectorInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InjectorInner")
            .field("parent", &self.parent.is_some())
            .field("providers", &5)
            .field("instances", &2)
            .finish()
    }
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Injector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Injector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl Injector {
    pub fn root() -> Self {
        Self {
            inner: Shared::new(InjectorInner {
                parent: None,
                providers: Store::new(HashMap::new()),
                instances: Store::new(HashMap::new()),
            }),
        }
    }

    pub fn child(parent: Shared<Injector>) -> Self {
        Self {
            inner: Shared::new(InjectorInner {
                parent: Some(parent.inner.clone()),
                providers: Store::new(HashMap::new()),
                instances: Store::new(HashMap::new()),
            }),
        }
    }
}

impl Clone for Injector {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/* ============================================================
 * Provider registration
 * ============================================================
 */

impl Injector {
    pub fn provide<T: ?Sized + 'static>(&self, provider: Provider) {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "thread-safe")]
        {
            self.inner
                .providers
                .write()
                .unwrap()
                .insert(type_id, provider);
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            self.inner.providers.borrow_mut().insert(type_id, provider);
        }
    }
}

/* ============================================================
 * Resolution
 * ============================================================
 */

// impl Injector {
//     pub fn resolve<T: 'static>(&self) -> Shared<T> {
//         let type_id = TypeId::of::<T>();

//         if let Some(instance) = self.get_instance(type_id) {
//             return instance.downcast::<T>().expect("Type mismatch in injector");
//         }

//         let provider = self.get_provider(type_id).expect("No provider found");

//         let instance = provider.create(self);

//         self.store_instance(type_id, instance.clone());

//         instance
//             .downcast::<T>()
//             .expect("Type mismatch in provider result")
//     }
// }

/* ============================================================
 * Internal helpers
 * ============================================================
 */

// impl Injector {
//     fn get_provider(&self, type_id: TypeId) -> Option<Provider> {
//         #[cfg(feature = "thread-safe")]
//         {
//             self.inner.providers.read().unwrap().get(&type_id).cloned()
//         }

//         #[cfg(not(feature = "thread-safe"))]
//         {
//             self.inner.providers.borrow().get(&type_id).cloned()
//         }
//     }

//     fn get_instance(&self, type_id: TypeId) -> Option<Shared<dyn Any>> {
//         #[cfg(feature = "thread-safe")]
//         {
//             self.inner.instances.read().unwrap().get(&type_id).cloned()
//         }

//         #[cfg(not(feature = "thread-safe"))]
//         {
//             self.inner.instances.borrow().get(&type_id).cloned()
//         }
//     }

//     fn store_instance(&self, type_id: TypeId, instance: Shared<dyn Any>) {
//         #[cfg(feature = "thread-safe")]
//         {
//             self.inner
//                 .instances
//                 .write()
//                 .unwrap()
//                 .insert(type_id, instance);
//         }

//         #[cfg(not(feature = "thread-safe"))]
//         {
//             self.inner.instances.borrow_mut().insert(type_id, instance);
//         }
//     }
// }
