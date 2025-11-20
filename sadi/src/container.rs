
use std::any::TypeId;
use std::collections::HashMap;

#[cfg(feature = "thread-safe")]
use std::sync::{Arc, RwLock};

#[cfg(not(feature = "thread-safe"))]
use std::cell::RefCell;
#[cfg(not(feature = "thread-safe"))]
use std::rc::Rc;

use crate::{Error, FactoriesMap, Factory, IntoShared, Provider, Shared};

pub struct Container {
    factories: FactoriesMap,
}

#[cfg(feature = "thread-safe")]
impl Container {
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }

    pub fn bind_abstract<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + Send + Sync + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            false,
        )
    }

    pub fn bind_abstract_singleton<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + Send + Sync + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            true,
        )
    }

    pub fn bind_concrete<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized + Send + Sync,
        U: 'static,
        F: Fn(&Container) -> U + Send + Sync + 'static,
        Arc<U>: Into<Arc<T>>,
    {
        self.bind_abstract::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Arc::new(u).into()
        })
    }

    pub fn bind_concrete_singleton<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized + Send + Sync,
        U: 'static,
        F: Fn(&Container) -> U + Send + Sync + 'static,
        Arc<U>: Into<Arc<T>>,
    {
        self.bind_abstract_singleton::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Arc::new(u).into()
        })
    }

    pub fn bind_instance<T, R>(&self, instance: R) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
        R: IntoShared<T> + 'static,
    {
        let shared_instance: Shared<T> = instance.into_shared();
        self.bind_internal(
            Box::new(move |_c: &Container| shared_instance.clone()),
            true,
        )
    }

    fn bind_internal<T>(&self, provider: Provider<T>, singleton: bool) -> Result<(), Error>
    where
        T: ?Sized + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let mut map = self.factories.write().unwrap();

        if map.contains_key(&type_id) {
            return Err(Error::factory_already_registered(type_name, "factory"));
        }
        let factory: Factory<T> = Factory::new(provider, singleton);
        let boxed: Box<dyn std::any::Any + Send + Sync> = Box::new(factory);
        map.insert(type_id, boxed);
        Ok(())
    }

    pub fn resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let _guard = crate::ResolveGuard::push(type_name)?;

        let map = self.factories.read().unwrap();
        let boxed = match map.get(&type_id) {
            Some(b) => b,
            None => return Err(Error::service_not_registered(type_name, "factory")),
        };
        let factory = boxed
            .downcast_ref::<Factory<T>>()
            .ok_or_else(|| Error::type_mismatch(type_name))?;

        Ok(factory.provide(self))
    }

    pub fn has<T>(&self) -> bool
    where
        T: ?Sized + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let map = self.factories.read().unwrap();
        map.contains_key(&type_id)
    }
}

#[cfg(not(feature = "thread-safe"))]
impl Container {
    pub fn new() -> Self {
        Self {
            factories: RefCell::new(HashMap::new()),
        }
    }

    pub fn bind_abstract<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            false,
        )
    }

    pub fn bind_abstract_singleton<T, R, F>(&self, provider: F) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
        F: Fn(&Container) -> R + 'static,
    {
        self.bind_internal(
            Box::new(move |c: &Container| {
                let r = provider(c);
                r.into_shared()
            }),
            true,
        )
    }

    pub fn bind_concrete<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized,
        U: 'static,
        F: Fn(&Container) -> U + 'static,
        Rc<U>: Into<Rc<T>>,
    {
        self.bind_abstract::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Rc::new(u).into()
        })
    }

    pub fn bind_concrete_singleton<T, U, F>(&self, provider: F) -> Result<(), Error>
    where
        T: 'static + Sized,
        U: 'static,
        F: Fn(&Container) -> U + 'static,
        Rc<U>: Into<Rc<T>>,
    {
        self.bind_abstract_singleton::<T, _, _>(move |c: &Container| {
            let u = provider(c);
            Rc::new(u).into()
        })
    }

    pub fn bind_instance<T, R>(&self, instance: R) -> Result<(), Error>
    where
        T: ?Sized + 'static,
        R: IntoShared<T> + 'static,
    {
        let shared_instance: Shared<T> = instance.into_shared();
        self.bind_internal(
            Box::new(move |_c: &Container| shared_instance.clone()),
            true,
        )
    }

    fn bind_internal<T>(&self, provider: Provider<T>, singleton: bool) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let mut map = self.factories.borrow_mut();

        if map.contains_key(&type_id) {
            return Err(Error::factory_already_registered(type_name, "factory"));
        }
        let factory: Factory<T> = Factory::new(provider, singleton);
        let boxed: Box<dyn std::any::Any> = Box::new(factory);
        map.insert(type_id, boxed);
        Ok(())
    }

    pub fn resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let _guard = crate::ResolveGuard::push(type_name)?;

        let map = self.factories.borrow();
        let boxed = match map.get(&type_id) {
            Some(b) => b,
            None => return Err(Error::service_not_registered(type_name, "factory")),
        };
        let factory = boxed
            .downcast_ref::<Factory<T>>()
            .ok_or_else(|| Error::type_mismatch(type_name))?;

        Ok(factory.provide(self))
    }

    pub fn has<T>(&self) -> bool
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let map = self.factories.borrow();
        map.contains_key(&type_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct S(pub i32);

    #[test]
    fn bind_and_resolve_concrete() {
        let c = Container::new();
        c.bind_concrete::<S, S, _>(|_c| S(7)).unwrap();
        let s = c.resolve::<S>().unwrap();
        assert_eq!((*s).0, 7);
    }

    #[test]
    fn bind_instance_and_singleton_behavior() {
        let c = Container::new();
        let instance = Shared::new(S(5));
        c.bind_instance::<S, _>(instance).unwrap();
        assert!(c.has::<S>());

        let a = c.resolve::<S>().unwrap();
        let b = c.resolve::<S>().unwrap();
        let pa = (&*a) as *const S;
        let pb = (&*b) as *const S;
        assert_eq!(pa, pb);
    }

    #[test]
    fn resolve_guard_detects_cycle() {
        let _g1 = crate::ResolveGuard::push("A").unwrap();
        let _g2 = crate::ResolveGuard::push("B").unwrap();
        let err = crate::ResolveGuard::push("A").unwrap_err();
        assert_eq!(err.kind, crate::ErrorKind::CircularDependency);
    }
}
