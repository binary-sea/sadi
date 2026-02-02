use axum::{Router, extract::State, routing::get};
use sadi::{Application, Injector, Module, Provider, Shared};
use std::net::SocketAddr;

trait NotificationProvider: Send + Sync {
    fn great(&self, message: &str);
}

struct StdOutNotificationProvider;

impl NotificationProvider for StdOutNotificationProvider {
    fn great(&self, message: &str) {
        println!("Hello from StdOutNotificationProvider: {}", message);
    }
}

pub struct AppModule;

impl Module for AppModule {
    fn providers(&self, injector: &Injector) {
        injector
            .provide::<dyn NotificationProvider>(Provider::root(|_| StdOutNotificationProvider));
    }
}

#[derive(Clone)]
pub struct AppState {
    pub app: Shared<Application>,
}

pub async fn notify_handler(State(state): State<AppState>) -> String {
    let injector = state.app.injector();
    let provider = injector.resolve::<dyn NotificationProvider>();
    provider.great("This is a notification from the handler!");
    "Notification sent!".to_string()
}

#[tokio::main]
async fn main() {
    let mut app = Application::new(AppModule);
    app.bootstrap();

    let state = AppState {
        app: Shared::new(app),
    };

    let router = Router::new()
        .route("/notify", get(notify_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("✅ Dependências resolvidas com sucesso!");
    println!("🚀 Servidor iniciado em http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, router).await.expect("Server error");
}
