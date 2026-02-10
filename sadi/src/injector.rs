use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::error::Error;
use crate::instance::Instance;
use crate::provider::Provider;
use crate::resolve_guard::ResolveGuard;
use crate::runtime::{Shared, Store};
use crate::scope::Scope;

pub struct Injector {
    inner: Shared<InjectorInner>,
}

struct InjectorInner {
    pub(crate) parent: Option<Shared<InjectorInner>>,

    #[cfg(not(feature = "thread-safe"))]
    pub(crate) providers: Store<HashMap<TypeId, Shared<dyn Any>>>,
    #[cfg(not(feature = "thread-safe"))]
    pub(crate) instances: Store<HashMap<TypeId, Shared<dyn Any>>>,

    #[cfg(feature = "thread-safe")]
    pub(crate) providers: Store<HashMap<TypeId, Shared<dyn Any + Send + Sync>>>,
    #[cfg(feature = "thread-safe")]
    pub(crate) instances: Store<HashMap<TypeId, Shared<dyn Any + Send + Sync>>>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for InjectorInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InjectorInner")
            .field("parent", &self.parent.is_some())
            .field("providers", &self.providers)
            .field("instances", &self.instances)
            .finish()
    }
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Injector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("inner", &self.inner)
            .finish()
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

    pub(crate) fn root_injector(&self) -> Injector {
        let mut current = self.clone();

        while let Some(parent) = &current.inner.parent {
            current = Injector {
                inner: parent.clone(),
            };
        }

        current
    }
}

#[cfg(not(feature = "thread-safe"))]
impl Injector {
    pub fn try_provide<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        match provider.scope {
            Scope::Root => {
                let root = self.root_injector();
                root.store_provider::<T>(provider)
            }

            Scope::Module | Scope::Transient => self.store_provider::<T>(provider),
        }
    }

    pub fn provide<T>(&self, provider: Provider<T>) -> &Self
    where
        T: ?Sized + 'static,
    {
        self.try_provide::<T>(provider).unwrap();
        self
    }

    pub(crate) fn get_provider<T>(&self) -> Option<Shared<dyn Any>>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        let local = self.inner.providers.borrow().get(&type_id).cloned();

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_provider::<T>();
        }

        None
    }

    pub(crate) fn resolve_provider<T>(&self) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider::<T>()
            .ok_or_else(|| Error::service_not_provided(type_name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        Ok(provider)
    }

    pub(crate) fn resolve_instance<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        let provider_ref = self.resolve_provider::<T>()?;

        Ok(Shared::new((provider_ref.factory)(self)))
    }

    pub(crate) fn store_instance<T>(&self, instance: Shared<Instance<T>>)
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        self.inner.instances.borrow_mut().insert(type_id, instance);
    }

    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let mut providers = self.inner.providers.borrow_mut();
        if providers.contains_key(&type_id) {
            return Err(Error::provider_already_registered(
                type_name,
                provider.scope.to_string().as_str(),
            ));
        }
        providers.insert(type_id, Shared::new(provider));

        Ok(())
    }

    pub(crate) fn get_instance<T>(&self) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        let local = self.inner.instances.borrow().get(&type_id).cloned();

        if local.is_some() {
            return local.and_then(|instance| instance.downcast::<Instance<T>>().ok());
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_instance::<T>();
        }

        None
    }

    pub fn try_resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        let _guard = ResolveGuard::push(type_id)?;

        if let Some(instance) = self.get_instance::<T>() {
            return Ok(instance.value());
        }

        let provider = self.resolve_provider::<T>()?;

        let instance = self.resolve_instance::<T>()?;

        if provider.scope == Scope::Transient {
            return Ok(instance.value());
        }

        match provider.scope {
            Scope::Root => {
                let root = self.root_injector();
                root.store_instance::<T>(instance.clone());
            }

            Scope::Module => {
                self.store_instance::<T>(instance.clone());
            }

            Scope::Transient => unreachable!(),
        }

        Ok(instance.value())
    }

    pub fn resolve<T>(&self) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve::<T>().unwrap()
    }

    pub fn optional_resolve<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve::<T>().ok()
    }
}

#[cfg(feature = "thread-safe")]
impl Injector {
    pub fn try_provide<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        match provider.scope {
            Scope::Root => {
                let root = self.root_injector();
                root.store_provider::<T>(provider)
            }

            Scope::Module | Scope::Transient => self.store_provider::<T>(provider),
        }
    }

    pub fn provide<T>(&self, provider: Provider<T>) -> &Self
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_provide::<T>(provider).unwrap();
        self
    }

    pub(crate) fn get_provider<T>(&self) -> Option<Shared<dyn Any + Send + Sync>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        let local = self.inner.providers.read().unwrap().get(&type_id).cloned();

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_provider::<T>();
        }

        None
    }

    pub(crate) fn resolve_provider<T>(&self) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider::<T>()
            .ok_or_else(|| Error::service_not_provided(type_name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        Ok(provider)
    }

    pub(crate) fn resolve_instance<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let provider_ref = self.resolve_provider::<T>()?;

        Ok(Shared::new((provider_ref.factory)(self)))
    }

    pub(crate) fn store_instance<T>(&self, instance: Shared<Instance<T>>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        self.inner
            .instances
            .write()
            .unwrap()
            .insert(type_id, instance);
    }

    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let mut providers = self.inner.providers.write().unwrap();
        if providers.contains_key(&type_id) {
            return Err(Error::provider_already_registered(
                type_name,
                provider.scope.to_string().as_str(),
            ));
        }
        providers.insert(type_id, Shared::new(provider));

        Ok(())
    }

    pub(crate) fn get_instance<T>(&self) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        let local = self.inner.instances.read().unwrap().get(&type_id).cloned();

        if local.is_some() {
            return local.and_then(|instance| instance.downcast::<Instance<T>>().ok());
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_instance::<T>();
        }

        None
    }

    pub fn try_resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        let _guard = ResolveGuard::push(type_id)?;

        if let Some(instance) = self.get_instance::<T>() {
            return Ok(instance.value());
        }

        let provider = self.resolve_provider::<T>()?;

        let instance = self.resolve_instance::<T>()?;

        if provider.scope == Scope::Transient {
            return Ok(instance.value());
        }

        match provider.scope {
            Scope::Root => {
                let root = self.root_injector();
                root.store_instance::<T>(instance.clone());
            }

            Scope::Module => {
                self.store_instance::<T>(instance.clone());
            }

            Scope::Transient => unreachable!(),
        }

        Ok(instance.value())
    }

    pub fn resolve<T>(&self) -> Shared<T>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve::<T>().unwrap()
    }

    pub fn optional_resolve<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve::<T>().ok()
    }
}
