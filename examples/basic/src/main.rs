use sadi::application::Application;
use sadi::module::Module;

#[derive(Debug)]
struct UserModule;

impl Module for UserModule {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![]
    }

    fn providers(&self, injector: &sadi::injector::Injector) {
        // Register providers here
    }
}

fn main() {
    let mut app = Application::new(UserModule);
    app.bootstrap();
    let injector = app.injector();

    println!("Application: {:?}", app);
    println!("Injector initialized: {:?}", injector);

    println!("Hello world");
}
