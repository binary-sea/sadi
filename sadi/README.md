# SaDi - Semi-automatic Dependency Injector

[![Crates.io](https://img.shields.io/crates/v/sadi.svg)](https://crates.io/crates/sadi)
[![Documentation](https://docs.rs/sadi/badge.svg)](https://docs.rs/sadi)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/JoaoPedro61/sadi/actions/workflows/dispatch.yml/badge.svg)](https://github.com/JoaoPedro61/sadi/actions/workflows/dispatch.yml)

A lightweight, type-safe dependency injection container for Rust applications. SaDi provides both transient and singleton service registration with automatic dependency resolution and circular dependency detection.

## ✨ Features

- 🔒 **Type-Safe**: Leverages Rust's type system for compile-time safety
- 🔄 **Transient Services**: Create new instances on each request
- 🔗 **Singleton Services**: Shared instances with reference counting
- 🔍 **Circular Detection**: Prevents infinite loops in dependency graphs
- ❌ **Error Handling**: Comprehensive error types with detailed messages
- 📊 **Optional Logging**: Tracing integration with feature gates
- 🚀 **Zero-Cost Abstractions**: Feature gates enable compile-time optimization

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
sadi = "0.1.0"

# Optional: Enable tracing support
sadi = { version = "0.1.0", features = ["tracing"] }
```

## 🚀 Quick Start

```rust
use sadi::SaDi;
use std::rc::Rc;

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
    db: Rc<DatabaseService>,
}

impl UserService {
    fn new(db: Rc<DatabaseService>) -> Self {
        Self { db }
    }
    
    fn create_user(&self, name: &str) -> String {
        self.db.query(&format!("INSERT INTO users (name) VALUES ('{}')", name))
    }
}

fn main() {
    // Set up the dependency injection container
    let container = SaDi::new()
        .factory_singleton(|_| DatabaseService::new())
        .factory(|di| UserService::new(di.get_singleton::<DatabaseService>()));

    // Use your services
    let user_service = container.get::<UserService>();
    println!("{}", user_service.create_user("Alice"));
}
```

## 📖 Usage Guide

### Service Registration

#### Transient Services
Create new instances on each request:

```rust
use sadi::SaDi;

struct LoggerService {
    session_id: String,
}

let container = SaDi::new()
    .factory(|_| LoggerService {
        session_id: uuid::Uuid::new_v4().to_string()
    });

// Each call creates a new logger with different session_id
let logger1 = container.get::<LoggerService>();
let logger2 = container.get::<LoggerService>();
```

#### Singleton Services
Create once and share across all dependents:

```rust
use sadi::SaDi;
use std::rc::Rc;

struct ConfigService {
    app_name: String,
    debug: bool,
}

let container = SaDi::new()
    .factory_singleton(|_| ConfigService {
        app_name: "MyApp".to_string(),
        debug: true,
    });

// Both calls return the same instance
let config1 = container.get_singleton::<ConfigService>();
let config2 = container.get_singleton::<ConfigService>();
assert_eq!(Rc::as_ptr(&config1), Rc::as_ptr(&config2));
```

### Error Handling

SaDi provides both panicking and non-panicking variants:

```rust
use sadi::{SaDi, Error};

let container = SaDi::new()
    .factory(|_| "Hello".to_string());

// Panicking version (use when you're sure service exists)
let service = container.get::<String>();

// Non-panicking version (returns Result)
match container.try_get::<String>() {
    Ok(service) => println!("Got: {}", service),
    Err(err) => println!("Error: {}", err),
}

// Trying to get unregistered service
match container.try_get::<u32>() {
    Ok(_) => unreachable!(),
    Err(Error { kind, message }) => {
        println!("Error kind: {:?}", kind);
        println!("Message: {}", message);
    }
}
```

### Dependency Injection

Services can depend on other services:

```rust
use sadi::SaDi;
use std::rc::Rc;

struct DatabaseService { /* ... */ }
struct CacheService { /* ... */ }
struct UserRepository {
    db: Rc<DatabaseService>,
    cache: Rc<CacheService>,
}

impl UserRepository {
    fn new(db: Rc<DatabaseService>, cache: Rc<CacheService>) -> Self {
        Self { db, cache }
    }
}

let container = SaDi::new()
    .factory_singleton(|_| DatabaseService::new())
    .factory_singleton(|_| CacheService::new())
    .factory(|di| UserRepository::new(
        di.get_singleton::<DatabaseService>(),
        di.get_singleton::<CacheService>(),
    ));

let repo = container.get::<UserRepository>();
```

## 🔍 Advanced Features

### Circular Dependency Detection

SaDi automatically detects and prevents circular dependencies:

```rust
use sadi::SaDi;

// This will panic with detailed error message
let container = SaDi::new()
    .factory(|di| ServiceA::new(di.get::<ServiceB>()))
    .factory(|di| ServiceB::new(di.get::<ServiceA>()));

// This panics: "Circular dependency detected: ServiceA -> ServiceB -> ServiceA"
let service = container.get::<ServiceA>();
```

### Tracing Integration

Enable the `tracing` feature for automatic logging:

```toml
[dependencies]
sadi = { version = "0.1.0", features = ["tracing"] }
```

```rust
use sadi::SaDi;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let container = SaDi::new()  // Logs: "Creating new SaDi container"
        .factory_singleton(|_| DatabaseService::new());  // Logs registration
    
    let db = container.get_singleton::<DatabaseService>();  // Logs resolution
}
```

## 🧪 Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with tracing feature
cargo test --features tracing

# Run documentation tests
cargo test --doc

# Run example
cargo run --example basic
```

## 📁 Project Structure

```
sadi/
├── src/
│   ├── lib.rs          # Library entry point
│   └── sadi.rs         # Core DI container implementation
├── examples/
│   └── basic/          # Comprehensive usage example
└── README.md           # This file
```

## 🔧 Configuration

### Feature Flags

- `tracing` - Enable tracing/logging support (optional)

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
git clone https://github.com/JoaoPedro61/sadi.git
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

### 🔄 Async Support
- [ ] **Async Factory Functions**: Support for `async fn` factories
- [ ] **Async Service Resolution**: Non-blocking service creation
- [ ] **Async Singleton Caching**: Thread-safe async singleton management
- [ ] **Async Circular Detection**: Proper handling in async contexts

### 🧵 Thread Safety
- [ ] **Arc-based Container**: Thread-safe version of SaDi using `Arc` instead of `Rc`
- [ ] **Send + Sync Services**: Support for services that implement `Send + Sync`
- [ ] **Concurrent Access**: Multiple threads accessing services simultaneously
- [ ] **Lock-free Operations**: Minimize contention in high-concurrency scenarios

### 🔧 Advanced Features
- [ ] **Service Scoping**: Request-scoped, thread-scoped service lifetimes
- [ ] **Lazy Initialization**: Defer singleton creation until first access
- [ ] **Service Decorators**: Middleware/decoration patterns for services
- [ ] **Conditional Registration**: Register services based on runtime conditions
- [ ] **Service Health Checks**: Built-in health monitoring for services
- [ ] **Service Metrics**: Performance and usage statistics
- [ ] **Hot Reloading**: Dynamic service replacement without container restart

### 📦 Ecosystem Integration
- [ ] **Tokio Integration**: First-class support for Tokio runtime
- [ ] **Actix-web Plugin**: Direct integration with Actix-web framework
- [ ] **Axum Integration**: Support for Axum web framework
- [ ] **Tower Service**: Implement Tower service trait
- [ ] **Serde Support**: Serialize/deserialize container configuration

### 🛠️ Developer Experience
- [ ] **Derive Macros**: Auto-generate factory functions from service structs
- [ ] **Builder Validation**: Compile-time validation of dependency graphs
- [ ] **Error Suggestions**: Better error messages with fix suggestions
- [ ] **IDE Integration**: Language server support for dependency analysis
- [ ] **Container Visualization**: Graphical representation of service dependencies

### 🔒 Security & Reliability
- [ ] **Service Isolation**: Sandboxing for untrusted services
- [ ] **Resource Limits**: Memory and CPU limits per service
- [ ] **Graceful Shutdown**: Proper cleanup on container disposal
- [ ] **Fault Tolerance**: Circuit breaker pattern for failing services

### 📊 Observability
- [ ] **OpenTelemetry**: Built-in telemetry and distributed tracing
- [ ] **Prometheus Metrics**: Expose container metrics for monitoring
- [ ] **Service Discovery**: Integration with service discovery systems
- [ ] **Health Endpoints**: HTTP endpoints for container health checks

### 🎯 Performance
- [ ] **Compile-time DI**: Zero-runtime-cost dependency injection
- [ ] **Service Pooling**: Object pooling for expensive-to-create services
- [ ] **Memory Optimization**: Reduced memory footprint for large containers
- [ ] **SIMD Optimizations**: Vectorized operations where applicable

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/JoaoPedro61/sadi/blob/main/LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by dependency injection patterns from other languages and frameworks
- Built with ❤️ using Rust's powerful type system
- Thanks to the Rust community for excellent crates and documentation

---

**Made with ❤️ by [João Pedro Martins](https://github.com/JoaoPedro61)**
