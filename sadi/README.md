# SaDi - Semi-automatic Dependency Injector

[![Crates.io](https://img.shields.io/crates/v/sadi.svg)](https://crates.io/crates/sadi)
[![Documentation](https://docs.rs/sadi/badge.svg)](https://docs.rs/sadi)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/binary-sea/sadi/actions/workflows/CI.yml/badge.svg)](https://github.com/binary-sea/sadi/actions/workflows/CI.yml)

A lightweight, type-safe dependency injection container for Rust applications. SaDi provides ergonomic service registration (including trait-object bindings), transient and singleton lifetimes, semi-automatic dependency resolution, and circular dependency detection.

## ✨ Features

- 🔒 **Type-Safe**: Leverages Rust's type system for compile-time safety
- 🔄 **Transient Services**: Create new instances on each request
- 🔗 **Singleton Services**: Shared instances with reference counting via `Arc` / `Rc`
- 🔍 **Circular Detection**: Prevents infinite loops in dependency graphs
- ❌ **Error Handling**: Comprehensive error types with detailed messages
- 📊 **Optional Logging**: Tracing integration with feature gates
- 🚀 **Zero-Cost Abstractions**: Feature gates enable compile-time optimization
- 🧵 **Thread-Safe by Default**: Uses `Arc` + `RwLock` for concurrent access
- 📦 **Module System**: Organize services into reusable modules
- 🏗️ **Enterprise Ready**: Supports layered architecture, repository pattern, and use cases

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
sadi = { path = "../sadi" }  # For local development
```

Or from crates.io (when published):

```toml
[dependencies]
sadi = "1.0.0"
```

## 🚀 Quick Start

```rust
use sadi::{Injector, Provider, Shared, Module, Application};

// Define your services
struct DatabaseService {
    connection_string: String,
}

impl DatabaseService {
    fn new() -> Self {
        Self {
            connection_string: "postgresql://localhost:5432/myapp".to_string(),
        }
    }

    fn query(&self, sql: &str) -> String {
        format!("Executing '{}' on {}", sql, self.connection_string)
    }
}

struct UserService {
    db: Shared<DatabaseService>,
}

impl UserService {
    fn new(db: Shared<DatabaseService>) -> Self {
        Self { db }
    }

    fn create_user(&self, name: &str) -> String {
        self.db.query(&format!("INSERT INTO users (name) VALUES ('{}')", name))
    }
}

struct RootModule;

impl Module for RootModule {
    fn providers(&self, injector: &sadi::Injector) {
         // Register DatabaseService as singleton
        injector.provide::<DatabaseService>(Provider::root(|_| {
            Shared::new(DatabaseService::new())
        }));
        
        // Register UserService with DatabaseService dependency
        injector.provide::<UserService>(Provider::root(|inj| {
            let db = inj.resolve::<DatabaseService>();
            UserService::new(db).into()
        }));
    }
}

fn main() {
    // Create an application and register services
    let mut app = Application::new(RootModule);

    app.bootstrap();

    // Resolve and use services
    match app.injector().try_resolve::<UserService>() {
        Ok(user_service) => println!("{}", user_service.create_user("Alice")),
        Err(e) => eprintln!("Service resolution failed: {}", e),
    }

    // or just
    app.injector().resolve::<UserService>(); // This panics if not registered
}
```

## � Examples

SaDi includes three comprehensive examples showcasing different use cases and patterns:

### 1. Basic Example
**Location:** `examples/basic/`

A simple introduction to SaDi fundamentals:
- Service registration with `Injector` and `Provider`
- Transient and singleton lifetimes
- Basic dependency resolution with `try_resolve()`
- Error handling with `Result` types

**Run:**
```bash
cargo run --example basic
```

### 2. Complex Example (Advanced Patterns)
**Location:** `examples/complex/`

Demonstrates enterprise-grade architecture with:
- **Domain Layer**: Clear entity definitions and repository interfaces
- **Application Layer**: Use case pattern for business logic
- **Infrastructure Layer**: SQLite persistence with concrete implementations
- **Dependency Injection**: Multi-level service composition
- **Module System**: Modular DI configuration with imported modules

Architecture:
```
core/
  ├── domain/       (User, Todo entities & repository traits)
  └── application/  (CreateUserUseCase, GetAllTodoUseCase, etc.)
infra/
  ├── di/           (Modules & dependency registration)
  └── persistence/  (SQLite repositories)
```

**Run:**
```bash
cd examples/complex
cargo run
```

**Run Tests:**
```bash
cd examples/complex
./test.sh
```

### 3. Axum REST API Example
**Location:** `examples/axum/`

Real-world REST API integration with **Axum** web framework:
- HTTP handler functions with DI-resolved dependencies
- Structured JSON responses with error handling
- CRUD endpoints for Users and Todos
- Service state management via Axum's `State` extractor
- Dependency resolution per-request

**Features:**
- `POST /users` - Create user
- `GET /users` - List all users
- `GET /users/{id}` - Get user by ID
- `DELETE /users/{id}` - Delete user
- `POST /todos` - Create todo
- `GET /todos` - List all todos
- `PUT /todos/{id}/status` - Update todo status
- `DELETE /todos/{id}` - Delete todo

**Run:**
```bash
# Terminal 1: Start server
cd examples/axum
cargo run

# Terminal 2: Run comprehensive test suite
cd examples/axum
./test.sh
```

The test suite includes:
- Server health checks
- Sequential dependency extraction between requests
- HTTP status code validation
- JSON response parsing and assertion

## �📖 Usage Guide

### Service Registration

#### Transient Services
Create new instances on each request:

```rust
use sadi::{Injector, Provider, Shared};
use uuid::Uuid;

struct LoggerService {
    session_id: String,
}

let injector = Injector::new();

// Transient: new instance each time (default behavior)
injector.provide::<LoggerService>(Provider::transient(|_| {
    Shared::new(LoggerService { 
        session_id: Uuid::new_v4().to_string() 
    })
}));

let logger1 = injector.resolve::<LoggerService>();
let logger2 = injector.resolve::<LoggerService>();
// logger1 and logger2 are different instances
```

#### Singleton Services
Create once and share across all dependents:

```rust
use sadi::{Injector, Provider, Shared};

struct ConfigService {
    app_name: String,
    debug: bool,
}

let injector = Injector::new();

// Singleton: same instance every time
injector.provide::<ConfigService>(Provider::root(|_| {
    Shared::new(ConfigService { 
        app_name: "MyApp".to_string(), 
        debug: true 
    })
}));

let config1 = injector.resolve::<ConfigService>();
let config2 = injector.resolve::<ConfigService>();
// config1 and config2 point to the same instance
```

### Error Handling

SaDi provides both panicking and non-panicking variants:

```rust
use sadi::{Injector, Provider, Shared, Error};

let injector = Injector::new();
injector.provide::<String>(Provider::new(|_| Shared::new("Hello".to_string())));

// Non-panicking (try_resolve returns Result)
match injector.try_resolve::<String>() {
    Ok(s) => println!("Got: {}", s),
    Err(e) => println!("Error: {}", e),
}

// Trying to resolve an unregistered type
match injector.try_resolve::<u32>() {
    Ok(_) => unreachable!(),
    Err(e) => println!("Expected error: {}", e),
}
```

### Dependency Injection

Services can depend on other services. Use module-based registration for clean organization:

```rust
use sadi::{Injector, Module, Provider, Shared};

struct DatabaseService { /* ... */ }
impl DatabaseService { fn new() -> Self { DatabaseService {} } }

struct CacheService { /* ... */ }
impl CacheService { fn new() -> Self { CacheService {} } }

struct UserRepository {
    db: Shared<DatabaseService>,
    cache: Shared<CacheService>,
}

impl UserRepository {
    fn new(db: Shared<DatabaseService>, cache: Shared<CacheService>) -> Self {
        Self { db, cache }
    }
}

// Define a module for persistence services
struct PersistenceModule;

impl Module for PersistenceModule {
    fn providers(&self, injector: &Injector) {
        injector.provide::<DatabaseService>(Provider::root(|_| {
            Shared::new(DatabaseService::new())
        }));
        
        injector.provide::<CacheService>(Provider::root(|_| {
            Shared::new(CacheService::new())
        }));
        
        injector.provide::<UserRepository>(Provider::root(|inj| {
            let db = inj.resolve::<DatabaseService>();
            let cache = inj.resolve::<CacheService>();
            UserRepository::new(db, cache).into()
        }));
    }
}

let injector = Injector::new();
let module = PersistenceModule;
module.providers(&injector);

let repo = injector.resolve::<UserRepository>();
```

## 🔍 Advanced Features

### Circular Dependency Detection

SaDi automatically detects and prevents circular dependencies by tracking resolution paths:

```rust
use sadi::{Injector, Provider, Shared};

// Example: attempting to create circular dependencies will fail
struct ServiceA {
    b: Shared<ServiceB>,
}

struct ServiceB {
    a: Shared<ServiceA>,
}

let injector = Injector::new();

// These registrations will create a circular dependency
// Attempting to resolve either service will result in an error
// Error: "Circular dependency detected in resolution path"
```

### Tracing Integration

Enable the `tracing` feature for automatic logging (the crate's `default` feature includes `tracing`):

```toml
[dependencies]
sadi = { path = "../sadi", features = ["tracing"] }
```

```rust
use sadi::{Application, Module, Provider, Shared};
use tracing::info;

struct MyModule;

impl Module for MyModule {
    fn providers(&self, injector: &sadi::Injector) {
        injector.provide::<DatabaseService>(Provider::root(|_| {
            info!("Registering DatabaseService");
            Shared::new(DatabaseService::new())
        }));
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut app = Application::new(MyModule);
    app.bootstrap();
    
    // Resolving services will be traced when tracing feature is enabled
    let _db = app.injector().try_resolve::<DatabaseService>();
}
```

## 🧪 Testing

### Unit Tests

Run the crate test suite:

```bash
# Run all tests for the workspace
cargo test

# Run tests for the sadi crate only
cargo test -p sadi

# Run with tracing feature
cargo test --features tracing

# Run documentation tests
cargo test --doc -p sadi
```

## 📁 Project Structure

```
sadi/
├── sadi/                 # SaDi library crate
│   ├── src/              # core implementation (container, macros, types)
│   └── README.md         # This file
├── examples/
│   ├── basic/            # Basic usage example with simple DI
│   ├── complex/          # Advanced DI patterns with SQLite, repositories, use cases
│   │   ├── src/
│   │   │   ├── core/     # Domain (entities, use cases)
│   │   │   └── infra/    # Infrastructure (persistence, DI configuration)
│   │   └── test.sh       # Test script for complex example
│   └── axum/             # REST API with Axum web framework
│       ├── src/
│       │   └── main.rs   # HTTP handlers with DI integration
│       └── test.sh       # Comprehensive API test suite
└── README.md
```

## 🔧 Configuration

### Feature Flags

SaDi exposes a small set of feature flags. See `sadi/Cargo.toml` for the authoritative list, but the crate currently defines:

- `thread-safe` (enabled by default) — switches internal shared pointer and synchronization primitives to `Arc` + `RwLock`/`Mutex` for thread-safe containers.
- `tracing` (enabled by default) — integrates with the `tracing` crate to emit logs during registration/resolution.

The workspace default enables both `thread-safe` and `tracing`. To opt out of thread-safe behavior (use `Rc` instead of `Arc`), disable the `thread-safe` feature.

### Environment Variables

When using the tracing feature, you can control logging levels:

```bash
# Set log level
RUST_LOG=debug cargo run --example basic

# Enable only SaDi logs
RUST_LOG=sadi=info cargo run --example basic
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

1. Clone the repository:
```bash
git clone https://github.com/binary-sea/sadi.git
cd sadi
```

2. Run tests:
```bash
cargo test --all-features
```

3. Check formatting:
```bash
cargo fmt --check
```

4. Run clippy:
```bash
cargo clippy -- -D warnings
```

## 📋 Roadmap & TODO

### 🧵 Thread Safety
- [x] **Arc-based Container**: Thread-safe version of SaDi using `Arc` instead of `Rc` (implemented behind the `thread-safe` feature)
- [x] **Send + Sync Services**: Support for `Send + Sync` services in thread-safe mode (enforced by API bounds)
- [x] **Concurrent Access**: Concurrent reads/writes supported via `RwLock`/`Mutex` in thread-safe mode
- [ ] **Lock-free Operations**: Minimize contention in high-concurrency scenarios

### 🔧 Advanced Features
- [x] **Lazy Initialization**: Singleton instances are created on first `provide` (implemented in `Factory`)
- [ ] **Service Metrics**: Internal container metrics for observability (resolution counts, timing)

### 📦 Ecosystem Integration
- [ ] **Async Factory Support**: Enable async/await in factory functions for Tokio/async-std runtimes
- [ ] **Actix-web Integration**: Extension trait and extractors for Actix-web framework
- [x] **Axum Integration**: Demonstrated with REST API example and state management
  - [ ] Create a plugin to automatically resolve dependency
- [ ] **Rocket Integration**: Layer and extractor support for Rocket web framework

### �️ Architectural Patterns
- [x] **Repository Pattern**: Demonstrated in complex example with SQLite repositories
- [x] **Layered Architecture**: Clean separation of domain, application, and infrastructure layers
- [x] **Use Case Pattern**: Business logic encapsulated in use cases with DI
- [x] **Web Framework Integration**: Explored with Axum web framework

### �🛠️ Developer Experience
- [ ] **Derive Macros**: Auto-generate factory functions from service structs (`#[injectable]`)
- [ ] **Error Suggestions**: Better error messages with fix suggestions

### 📊 Observability
- [ ] **OpenTelemetry**: Built-in telemetry and distributed tracing
- [ ] **Prometheus Metrics**: Expose container metrics for monitoring

### 🎯 Performance
- [ ] **Memory Optimization**: Reduced memory footprint for large containers

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/binary-sea/sadi/blob/main/LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by dependency injection patterns from other languages and frameworks
- Built with ❤️ using Rust's powerful type system
- Thanks to the Rust community for excellent crates and documentation

---

**SaDi** - A semi-automatic dependency injection container for Rust  
**Repository:** [binary-sea/sadi](https://github.com/binary-sea/sadi)  
**Made with ❤️ by the Binary Sea Team**
