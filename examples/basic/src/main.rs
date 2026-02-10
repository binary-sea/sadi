use sadi::{Injector, Provider, Shared};

// ============================================================
// SERVICE TYPES
// ============================================================

/// Configuration service - demonstrates singleton
#[derive(Clone, Debug)]
struct Config {
    database_url: String,
    environment: String,
}

impl Config {
    fn new() -> Self {
        Self {
            database_url: "postgresql://localhost/mydb".to_string(),
            environment: "development".to_string(),
        }
    }
}

/// Logger service - demonstrates singleton
#[derive(Debug)]
struct Logger {
    prefix: String,
}

impl Logger {
    fn new(config: &Config) -> Self {
        Self {
            prefix: format!("[{}]", config.environment),
        }
    }

    fn log(&self, message: &str) {
        println!("{} {}", self.prefix, message);
    }
}

/// Database connection - demonstrates singleton
#[derive(Debug)]
struct Database {
    _url: String,
    id: usize,
}

impl Database {
    fn new(config: &Config) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Self {
            _url: config.database_url.clone(),
            id,
        }
    }

    fn query(&self, sql: &str) -> String {
        format!("DB[{}]: {}", self.id, sql)
    }
}

/// User service - demonstrates singleton with dependencies
#[derive(Debug)]
struct UserService;

impl UserService {
    fn get_user(&self, db: &Database) -> String {
        db.query("SELECT * FROM users WHERE id = 1")
    }
}

/// Request handler - demonstrates transient (new instance each request)
#[derive(Debug)]
struct RequestHandler {
    id: u64,
}

impl RequestHandler {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Self {
            id: duration.as_nanos() as u64 % 10000,
        }
    }

    fn handle(&self, message: &str) -> String {
        format!("Request #{}: {}", self.id, message)
    }
}

/// Cache service - demonstrates singleton
#[derive(Debug)]
struct Cache {
    _name: String,
}

impl Cache {
    fn new() -> Self {
        Self {
            _name: "in-memory-cache".to_string(),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        Some(format!("Cached value for {}", key))
    }
}

// ============================================================
// MAIN
// ============================================================

fn main() {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║        SADI - Dependency Injection Container           ║");
    println!("║                  Complex Example                       ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // ────────────────────────────────────────────────────────────
    // Setup with direct Injector usage
    // ────────────────────────────────────────────────────────────
    println!("📦 Creating injector and registering services...");
    let injector = Injector::root();

    // Register singleton Config
    injector
        .try_provide::<Config>(Provider::singleton(|_injector| Shared::new(Config::new())))
        .expect("failed to register Config");

    // Register singleton Logger with Config dependency
    injector
        .try_provide::<Logger>(Provider::singleton(|injector| {
            let config = injector
                .try_resolve::<Config>()
                .expect("failed to resolve Config");
            Shared::new(Logger::new(&config))
        }))
        .expect("failed to register Logger");

    // Register singleton Database with Config dependency
    injector
        .try_provide::<Database>(Provider::singleton(|injector| {
            let config = injector
                .try_resolve::<Config>()
                .expect("failed to resolve Config");
            Shared::new(Database::new(&config))
        }))
        .expect("failed to register Database");

    // Register singleton UserService
    injector
        .try_provide::<UserService>(Provider::singleton(|_injector| Shared::new(UserService)))
        .expect("failed to register UserService");

    // Register singleton Cache
    injector
        .try_provide::<Cache>(Provider::singleton(|_injector| Shared::new(Cache::new())))
        .expect("failed to register Cache");

    // Register transient RequestHandler
    injector
        .try_provide::<RequestHandler>(Provider::transient(|_injector| {
            Shared::new(RequestHandler::new())
        }))
        .expect("failed to register RequestHandler");

    println!("✓ All services registered!\n");

    // ────────────────────────────────────────────────────────────
    // Example 1: Singleton Services (same instance)
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 1: Singleton Services (same instance)");
    println!("────────────────────────────────────────────────────────");
    let config1 = injector
        .try_resolve::<Config>()
        .expect("failed to resolve Config");
    let config2 = injector
        .try_resolve::<Config>()
        .expect("failed to resolve Config");
    println!("Config 1: {:?}", config1);
    println!("Config 2: {:?}", config2);
    let same = Shared::ptr_eq(&config1, &config2);
    println!("✓ Same instance: {}\n", same);

    // ────────────────────────────────────────────────────────────
    // Example 2: Dependency Resolution
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 2: Dependency Resolution");
    println!("────────────────────────────────────────────────────────");
    let db1 = injector
        .try_resolve::<Database>()
        .expect("failed to resolve Database");
    let db2 = injector
        .try_resolve::<Database>()
        .expect("failed to resolve Database");
    println!("Database 1: {:?}", db1);
    println!("Database 2: {:?}", db2);
    let same_db = Shared::ptr_eq(&db1, &db2);
    println!("✓ Same instance (singleton): {}\n", same_db);

    // ────────────────────────────────────────────────────────────
    // Example 3: Using Services with Dependencies
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 3: Using Services with Dependencies");
    println!("────────────────────────────────────────────────────────");
    let user_service = injector
        .try_resolve::<UserService>()
        .expect("failed to resolve UserService");
    let result = user_service.get_user(&db1);
    println!("UserService result: {}", result);

    let cache = injector
        .try_resolve::<Cache>()
        .expect("failed to resolve Cache");
    println!("Cache: {:?}", cache);
    if let Some(value) = cache.get("user:1") {
        println!("Cache lookup: {}", value);
    }
    println!();

    // ────────────────────────────────────────────────────────────
    // Example 4: Logger (Singleton with configuration)
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 4: Logger (Singleton with configuration)");
    println!("────────────────────────────────────────────────────────");
    let logger = injector
        .try_resolve::<Logger>()
        .expect("failed to resolve Logger");
    logger.log("Application started");
    logger.log("Services initialized");
    logger.log("Ready to handle requests");
    println!();

    // ────────────────────────────────────────────────────────────
    // Example 5: Transient Services (different instances)
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 5: Transient Services (different instances)");
    println!("────────────────────────────────────────────────────────");
    let handler1 = injector
        .try_resolve::<RequestHandler>()
        .expect("failed to resolve RequestHandler");
    let handler2 = injector
        .try_resolve::<RequestHandler>()
        .expect("failed to resolve RequestHandler");
    println!("Handler 1: {:?}", handler1);
    println!("Handler 2: {:?}", handler2);
    println!("Handler 1 message: {}", handler1.handle("Get user"));
    println!("Handler 2 message: {}", handler2.handle("Create post"));
    let different = !Shared::ptr_eq(&handler1, &handler2);
    println!("✓ Different instances (transient): {}\n", different);

    // ────────────────────────────────────────────────────────────
    // Example 6: Multiple requests with transient handlers
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 6: Multiple requests with transient handlers");
    println!("────────────────────────────────────────────────────────");
    for i in 1..=3 {
        let handler = injector
            .try_resolve::<RequestHandler>()
            .expect("failed to resolve RequestHandler");
        println!(
            "Request {}: {}",
            i,
            handler.handle(&format!("Operation {}", i))
        );
    }
    println!();

    // ────────────────────────────────────────────────────────────
    // Example 7: Verify singleton persistence
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 7: Verify singleton persistence across resolves");
    println!("────────────────────────────────────────────────────────");
    let config3 = injector
        .try_resolve::<Config>()
        .expect("failed to resolve Config");
    let db3 = injector
        .try_resolve::<Database>()
        .expect("failed to resolve Database");
    let cache1 = injector
        .try_resolve::<Cache>()
        .expect("failed to resolve Cache");
    let cache2 = injector
        .try_resolve::<Cache>()
        .expect("failed to resolve Cache");

    println!(
        "Config instances same: {}",
        Shared::ptr_eq(&config1, &config3)
    );
    println!("Database instances same: {}", Shared::ptr_eq(&db1, &db3));
    println!("Cache instances same: {}", Shared::ptr_eq(&cache1, &cache2));
    println!();

    // ────────────────────────────────────────────────────────────
    // Example 8: Service Composition
    // ────────────────────────────────────────────────────────────
    println!("────────────────────────────────────────────────────────");
    println!("Example 8: Service Composition");
    println!("────────────────────────────────────────────────────────");
    let logger = injector
        .try_resolve::<Logger>()
        .expect("failed to resolve Logger");
    let config = injector
        .try_resolve::<Config>()
        .expect("failed to resolve Config");
    let db = injector
        .try_resolve::<Database>()
        .expect("failed to resolve Database");

    logger.log(&format!("Database initialized at {}", config.database_url));
    logger.log(&db.query("SELECT COUNT(*) FROM users"));
    logger.log("Service composition working perfectly!");
    println!();

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete                    ║");
    println!("║                                                        ║");
    println!("║  Demonstrated Features:                                ║");
    println!("║  ✓ Injector & Provider pattern                         ║");
    println!("║  ✓ Singleton services (same instance)                  ║");
    println!("║  ✓ Transient services (different instances)            ║");
    println!("║  ✓ Service dependency resolution                       ║");
    println!("║  ✓ Multiple service types                              ║");
    println!("║  ✓ Service composition patterns                        ║");
    println!("╚════════════════════════════════════════════════════════╝");
}
