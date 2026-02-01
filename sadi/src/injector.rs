use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::error::Error;
use crate::provider::Provider;
use crate::resolve_guard::ResolveGuard;
use crate::runtime::{Shared, Store};
use crate::scope::Scope;

pub struct Injector {
    inner: Shared<InjectorInner>,
}

struct InjectorInner {
    parent: Option<Shared<InjectorInner>>,
    providers: Store<HashMap<TypeId, Shared<Provider>>>,
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

impl Injector {
    pub fn try_provide<T: ?Sized + 'static>(&self, provider: Provider) -> Result<(), Error> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let scope_label = match provider.scope {
            Scope::Root => "root",
            Scope::Module => "module",
            Scope::Transient => "transient",
        };

        #[cfg(feature = "thread-safe")]
        {
            let mut providers = self.inner.providers.write().unwrap();
            if providers.contains_key(&type_id) {
                return Err(Error::provider_already_registered(type_name, scope_label));
            }
            providers.insert(type_id, Shared::new(provider));
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            let mut providers = self.inner.providers.borrow_mut();
            if providers.contains_key(&type_id) {
                return Err(Error::provider_already_registered(type_name, scope_label));
            }
            providers.insert(type_id, Shared::new(provider));
        }

        Ok(())
    }

    pub fn provide<T: ?Sized + 'static>(&self, provider: Provider) -> &Self {
        self.try_provide::<T>(provider).unwrap();
        self
    }

    pub fn try_resolve<T: 'static>(&self) -> Result<Shared<T>, Error> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let _guard = ResolveGuard::push(type_id)?;

        if let Some(instance) = self.get_instance(type_id) {
            return instance
                .downcast::<T>()
                .map_err(|_| Error::type_mismatch(type_name));
        }

        let provider = self
            .get_provider(type_id)
            .ok_or_else(|| Error::service_not_provided(type_name))?;

        if provider.scope == Scope::Transient {
            let instance = (provider.factory)(self);
            return instance
                .downcast::<T>()
                .map_err(|_| Error::type_mismatch(type_name));
        }

        let instance = (provider.factory)(self);
        let typed = instance
            .clone()
            .downcast::<T>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        match provider.scope {
            Scope::Root => {
                let root = self.root_injector();
                root.store_instance(type_id, instance.clone());
            }

            Scope::Module => {
                self.store_instance(type_id, instance.clone());
            }

            Scope::Transient => unreachable!(),
        }

        Ok(typed)
    }

    pub fn resolve<T: 'static>(&self) -> Shared<T> {
        self.try_resolve::<T>().unwrap()
    }

    pub fn optional_resolve<T: 'static>(&self) -> Option<Shared<T>> {
        self.try_resolve::<T>().ok()
    }

    fn get_provider(&self, type_id: TypeId) -> Option<Shared<Provider>> {
        let local = {
            #[cfg(feature = "thread-safe")]
            {
                self.inner.providers.read().unwrap().get(&type_id).cloned()
            }

            #[cfg(not(feature = "thread-safe"))]
            {
                self.inner.providers.borrow().get(&type_id).cloned()
            }
        };

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_provider(type_id);
        }

        None
    }

    fn get_instance(&self, type_id: TypeId) -> Option<Shared<dyn Any>> {
        let local = {
            #[cfg(feature = "thread-safe")]
            {
                self.inner.instances.read().unwrap().get(&type_id).cloned()
            }

            #[cfg(not(feature = "thread-safe"))]
            {
                self.inner.instances.borrow().get(&type_id).cloned()
            }
        };

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_instance(type_id);
        }

        None
    }

    fn store_instance(&self, type_id: TypeId, instance: Shared<dyn Any>) {
        #[cfg(feature = "thread-safe")]
        {
            self.inner
                .instances
                .write()
                .unwrap()
                .insert(type_id, instance);
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            self.inner.instances.borrow_mut().insert(type_id, instance);
        }
    }

    fn root_injector(&self) -> Injector {
        let mut current = self.clone();

        while let Some(parent) = &current.inner.parent {
            current = Injector {
                inner: parent.clone(),
            };
        }

        current
    }
}
