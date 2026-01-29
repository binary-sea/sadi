use crate::scope::Scope;
use std::any::Any;
#[cfg(not(feature = "thread-safe"))]
use std::rc::Rc;
#[cfg(feature = "thread-safe")]
use std::sync::Arc;

pub struct Provider {
    pub scope: Scope,

    #[cfg(not(feature = "thread-safe"))]
    pub factory: Box<dyn Fn() -> Rc<dyn Any> + 'static>,
    #[cfg(feature = "thread-safe")]
    pub factory: Box<dyn Fn() -> Arc<dyn Any + Send + Sync> + Send + Sync + 'static>,
}

#[cfg(feature = "thread-safe")]
impl Provider {
    pub fn singleton<T, F>(factory: F) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            scope: Scope::Module,
            factory: Box::new(move || Arc::new(factory()) as Arc<dyn Any + Send + Sync>),
        }
    }

    pub fn transient<T, F>(factory: F) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            scope: Scope::Transient,
            factory: Box::new(move || Arc::new(factory()) as Arc<dyn Any + Send + Sync>),
        }
    }
}

#[cfg(not(feature = "thread-safe"))]
impl Provider {
    pub fn singleton<T, F>(factory: F) -> Self
    where
        T: Any + 'static,
        F: Fn() -> T + 'static,
    {
        Self {
            scope: Scope::Module,
            factory: Box::new(move || Rc::new(factory()) as Rc<dyn Any>),
        }
    }

    pub fn transient<T, F>(factory: F) -> Self
    where
        T: Any + 'static,
        F: Fn() -> T + 'static,
    {
        Self {
            scope: Scope::Transient,
            factory: Box::new(move || Rc::new(factory()) as Rc<dyn Any>),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "thread-safe"))]
    use std::rc::Rc;

    #[cfg(feature = "thread-safe")]
    use std::sync::Arc;

    #[test]
    fn should_create_singleton_provider_with_module_scope() {
        let provider = Provider::singleton(|| 42u32);

        assert!(matches!(provider.scope, Scope::Module));
    }

    #[test]
    fn should_create_transient_provider_with_transient_scope() {
        let provider = Provider::transient(|| "hello".to_string());

        assert!(matches!(provider.scope, Scope::Transient));
    }

    #[test]
    fn singleton_factory_should_return_valid_value() {
        let provider = Provider::singleton(|| 100u32);

        let value = (provider.factory)();

        #[cfg(not(feature = "thread-safe"))]
        {
            let value = value.downcast_ref::<u32>().unwrap();
            assert_eq!(*value, 100);
        }

        #[cfg(feature = "thread-safe")]
        {
            let value = value.downcast_ref::<u32>().unwrap();
            assert_eq!(*value, 100);
        }
    }

    #[test]
    fn transient_factory_should_create_new_instances() {
        let provider = Provider::transient(|| "transient".to_string());

        let first = (provider.factory)();
        let second = (provider.factory)();

        #[cfg(not(feature = "thread-safe"))]
        {
            assert!(!Rc::ptr_eq(
                &first.downcast::<String>().unwrap(),
                &second.downcast::<String>().unwrap()
            ));
        }

        #[cfg(feature = "thread-safe")]
        {
            assert!(!Arc::ptr_eq(
                &first.downcast::<String>().unwrap(),
                &second.downcast::<String>().unwrap()
            ));
        }
    }
}
