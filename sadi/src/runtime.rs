#[cfg(feature = "thread-safe")]
use std::sync::{Arc, RwLock};

#[cfg(not(feature = "thread-safe"))]
use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "thread-safe")]
pub type Shared<T> = Arc<T>;

#[cfg(not(feature = "thread-safe"))]
pub type Shared<T> = Rc<T>;

#[cfg(feature = "thread-safe")]
pub type Store<T> = RwLock<T>;

#[cfg(not(feature = "thread-safe"))]
pub type Store<T> = RefCell<T>;
