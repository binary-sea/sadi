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
}

pub fn build() -> Result<Application, String> {
    let mut app = Application::new(RootModule);

    // Register SqliteClient first
    app.injector().provide::<SqliteClient>(Provider::root(|_| {
        let client = SqliteClient::new().expect("Failed to load sqlite client");
        Shared::new(client)
    }));

    app.bootstrap();

    Ok(app)
}
