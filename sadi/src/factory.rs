use crate::{InstanceCell, Provider, Shared, Container};

pub struct Factory<T: ?Sized + 'static> {
    provider: Provider<T>,
    singleton: bool,
    instance: InstanceCell<T>,
}

impl<T: ?Sized + 'static> Factory<T> {
    pub fn new(provider: Provider<T>, singleton: bool) -> Self {
        Self {
            provider,
            singleton,
            instance: {
                #[cfg(feature = "thread-safe")]
                {
                    std::sync::Mutex::new(None)
                }
                #[cfg(not(feature = "thread-safe"))]
                {
                    std::cell::RefCell::new(None)
                }
            },
        }
    }

    pub fn provide(&self, container: &Container) -> Shared<T> {
        if self.singleton {
            // thread-safe branch
            #[cfg(feature = "thread-safe")]
            {
                let mut guard = self.instance.lock().unwrap();
                if let Some(inst) = guard.as_ref() {
                    return inst.clone();
                }
                let inst = (self.provider)(container);
                *guard = Some(inst.clone());
                inst
            }

            // non-thread-safe branch
            #[cfg(not(feature = "thread-safe"))]
            {
                let mut borrow = self.instance.borrow_mut();
                if let Some(inst) = borrow.as_ref() {
                    return inst.clone();
                }
                let inst = (self.provider)(container);
                *borrow = Some(inst.clone());
                inst
            }
        } else {
            (self.provider)(container)
        }
    }
}
