use crate::injector::Injector;
use crate::module::Module;
use crate::runtime::Shared;

pub struct Application {
    root: Option<Box<dyn Module>>,
    injector: Shared<Injector>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application")
            .field("injector", &"...")
            .field("root", &"<dyn Module>")
            .finish()
    }
}

impl Application {
    pub fn new(root: impl Module + 'static) -> Self {
        Self {
            root: Some(Box::new(root)),
            injector: Shared::new(Injector::root()),
        }
    }

    pub fn bootstrap(&mut self) {
        let root = self.root.take().expect("Application already bootstrapped");

        Self::load_module(self.injector.clone(), root);
    }

    pub fn injector(&self) -> Shared<Injector> {
        self.injector.clone()
    }

    pub fn is_bootstrapped(&self) -> bool {
        self.root.is_none()
    }

    fn load_module(parent: Shared<Injector>, module: Box<dyn Module>) {
        let module_injector = Shared::new(Injector::child(parent.clone()));

        for import in module.imports() {
            Self::load_module(module_injector.clone(), import);
        }

        module.providers(&module_injector);
    }
}
