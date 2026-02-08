use sadi::{Application, Module, Provider, Shared};

use crate::infra::persistence::sqlite::SqliteClient;

mod repositories;
mod use_cases;

pub use repositories::*;
pub use use_cases::*;

pub struct RootModule;

impl Module for RootModule {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![Box::new(RepositoriesModule), Box::new(UseCasesModule)]
    }

    fn providers(&self, _: &sadi::Injector) {}
}

pub fn build() -> Result<Application, String> {
    let app = Application::new(RootModule);

    // Register SqliteClient first
    app.injector().provide::<SqliteClient>(Provider::root(|_| {
        let client = SqliteClient::new().expect("Failed to load sqlite client");
        Shared::new(client)
    }));

    // Manually call providers from imported modules
    let repositories_module = RepositoriesModule;
    repositories_module.providers(&app.injector());
    
    let use_cases_module = UseCasesModule;
    use_cases_module.providers(&app.injector());

    Ok(app)
}
