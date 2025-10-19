use sadi::{Error, SaDi};
use std::rc::Rc;

// Example services to demonstrate dependency injection

/// A configuration service (singleton) - shared across the application
#[derive(Debug)]
struct ConfigService {
    app_name: String,
    version: String,
    debug_mode: bool,
}

impl ConfigService {
    fn new() -> Self {
        println!("⚙️  Initializing ConfigService (singleton)");
        Self {
            app_name: "SaDi Example App".to_string(),
            version: "1.0.0".to_string(),
            debug_mode: true,
        }
    }

    fn get_info(&self) -> String {
        format!(
            "{} v{} (debug: {})",
            self.app_name, self.version, self.debug_mode
        )
    }
}

/// A database service (singleton) - expensive to create, shared across services
#[derive(Debug)]
struct DatabaseService {
    connection_string: String,
    connected: bool,
}

impl DatabaseService {
    fn new(config: Rc<ConfigService>) -> Self {
        println!("🗄️  Connecting to database...");
        Self {
            connection_string: format!(
                "postgresql://localhost:5432/{}",
                config.app_name.to_lowercase()
            ),
            connected: true,
        }
    }

    fn execute_query(&self, query: &str) -> String {
        if self.connected {
            format!("✅ Executed: '{}' on {}", query, self.connection_string)
        } else {
            "❌ Database not connected".to_string()
        }
    }
}

/// A logger service (transient) - new instance per injection
#[derive(Debug)]
struct LoggerService {
    db: Rc<DatabaseService>,
    config: Rc<ConfigService>,
}

impl LoggerService {
    fn new(db: Rc<DatabaseService>, config: Rc<ConfigService>) -> Self {
        println!("📝 Creating LoggerService instance");
        Self { db, config }
    }

    fn log(&self, level: &str, message: &str) {
        let log_entry = format!("[{}] {}", level, message);

        if self.config.debug_mode {
            println!("🔍 Debug Log: {}", log_entry);
        }

        // Store log in database
        let result = self.db.execute_query(&format!(
            "INSERT INTO logs (level, message) VALUES ('{}', '{}')",
            level, message
        ));
        println!("📊 {}", result);
    }
}

/// A user service (transient) - business logic layer
#[derive(Debug)]
struct UserService {
    db: Rc<DatabaseService>,
    logger: LoggerService,
}

impl UserService {
    fn new(db: Rc<DatabaseService>, logger: LoggerService) -> Self {
        println!("👤 Creating UserService instance");
        Self { db, logger }
    }

    fn create_user(&self, name: &str, email: &str) -> Result<String, String> {
        self.logger
            .log("INFO", &format!("Creating user: {} ({})", name, email));

        // Simulate user creation
        let query = format!(
            "INSERT INTO users (name, email) VALUES ('{}', '{}')",
            name, email
        );
        let result = self.db.execute_query(&query);

        if result.contains("✅") {
            let success_msg = format!("User '{}' created successfully", name);
            self.logger.log("INFO", &success_msg);
            Ok(success_msg)
        } else {
            let error_msg = format!("Failed to create user '{}'", name);
            self.logger.log("ERROR", &error_msg);
            Err(error_msg)
        }
    }

    fn get_user(&self, id: u32) -> String {
        self.logger
            .log("INFO", &format!("Retrieving user with ID: {}", id));
        self.db
            .execute_query(&format!("SELECT * FROM users WHERE id = {}", id))
    }
}

/// An email service (transient) - external service integration
#[derive(Debug)]
struct EmailService {
    config: Rc<ConfigService>,
    logger: LoggerService,
}

impl EmailService {
    fn new(config: Rc<ConfigService>, logger: LoggerService) -> Self {
        println!("📧 Creating EmailService instance");
        Self { config, logger }
    }

    fn send_welcome_email(&self, user_name: &str, email: &str) -> bool {
        self.logger
            .log("INFO", &format!("Sending welcome email to {}", email));

        if self.config.debug_mode {
            println!("📧 [DEBUG] Would send email to: {}", email);
            println!("📧 [DEBUG] Subject: Welcome to {}!", self.config.app_name);
            println!(
                "📧 [DEBUG] Body: Hello {}, welcome to our application!",
                user_name
            );
        }

        // Simulate email sending
        let success = email.contains("@");
        if success {
            self.logger.log(
                "INFO",
                &format!("Welcome email sent successfully to {}", email),
            );
        } else {
            self.logger.log(
                "ERROR",
                &format!("Failed to send email to invalid address: {}", email),
            );
        }

        success
    }
}

/// Application service (transient) - orchestrates other services
#[derive(Debug)]
struct ApplicationService {
    user_service: UserService,
    email_service: EmailService,
    config: Rc<ConfigService>,
}

impl ApplicationService {
    fn new(
        user_service: UserService,
        email_service: EmailService,
        config: Rc<ConfigService>,
    ) -> Self {
        println!("🚀 Creating ApplicationService instance");
        Self {
            user_service,
            email_service,
            config,
        }
    }

    fn register_user(&self, name: &str, email: &str) -> Result<String, String> {
        println!("\n--- User Registration Process ---");
        println!("🏢 Using {} for registration", self.config.get_info());

        // Create user
        match self.user_service.create_user(name, email) {
            Ok(result) => {
                // Send welcome email
                if self.email_service.send_welcome_email(name, email) {
                    Ok(format!("{} - Welcome email sent!", result))
                } else {
                    Ok(format!(
                        "{} - Warning: Failed to send welcome email",
                        result
                    ))
                }
            }
            Err(error) => Err(error),
        }
    }
}

fn main() -> Result<(), Error> {
    println!("🚀 SaDi Dependency Injection Example");
    println!("=====================================\n");

    // Create and configure the DI container
    println!("📦 Setting up dependency injection container...");

    let container = SaDi::new()
        // Register singleton services (expensive to create, shared state)
        .factory_singleton(|_| ConfigService::new())
        .factory_singleton(|di: &SaDi| DatabaseService::new(di.get_singleton::<ConfigService>()))
        // Register transient services (created fresh each time)
        .factory(|di: &SaDi| {
            LoggerService::new(
                di.get_singleton::<DatabaseService>(),
                di.get_singleton::<ConfigService>(),
            )
        })
        .factory(|di: &SaDi| {
            UserService::new(
                di.get_singleton::<DatabaseService>(),
                di.get::<LoggerService>(),
            )
        })
        .factory(|di: &SaDi| {
            EmailService::new(
                di.get_singleton::<ConfigService>(),
                di.get::<LoggerService>(),
            )
        })
        .factory(|di: &SaDi| {
            ApplicationService::new(
                di.get::<UserService>(),
                di.get::<EmailService>(),
                di.get_singleton::<ConfigService>(),
            )
        });

    println!("✅ Container configured successfully!\n");

    // Demonstrate singleton behavior
    println!("--- Singleton Behavior ---");
    let config1 = container.get_singleton::<ConfigService>();
    let config2 = container.get_singleton::<ConfigService>();

    println!("📋 Config1: {}", config1.get_info());
    println!("📋 Config2: {}", config2.get_info());
    println!(
        "🔄 Same instance? {}",
        Rc::as_ptr(&config1) == Rc::as_ptr(&config2)
    );

    // Use the application
    println!("\n--- Application Usage ---");
    let app = container.get::<ApplicationService>();

    // Register some users
    match app.register_user("Alice Johnson", "alice@example.com") {
        Ok(result) => println!("✅ {}", result),
        Err(error) => println!("❌ {}", error),
    }

    match app.register_user("Bob Smith", "bob@example.com") {
        Ok(result) => println!("✅ {}", result),
        Err(error) => println!("❌ {}", error),
    }

    // Try with invalid email
    match app.register_user("Charlie Brown", "invalid-email") {
        Ok(result) => println!("✅ {}", result),
        Err(error) => println!("❌ {}", error),
    }

    // Demonstrate transient behavior
    println!("\n--- Transient Behavior ---");
    let user_service1 = container.get::<UserService>();
    let user_service2 = container.get::<UserService>();

    println!(
        "🔄 Different UserService instances? {}",
        !std::ptr::eq(&user_service1, &user_service2)
    );

    // But they share the same database singleton
    println!(
        "🔄 Same database instance? {}",
        Rc::as_ptr(&user_service1.db) == Rc::as_ptr(&user_service2.db)
    );

    // Show some queries
    println!("\n--- Database Queries ---");
    println!("📊 {}", user_service1.get_user(1));
    println!("📊 {}", user_service2.get_user(2));

    // Demonstrate error handling
    println!("\n--- Error Handling ---");

    // Try to get a service that wasn't registered
    match container.try_get::<String>() {
        Ok(_) => println!("This shouldn't happen"),
        Err(e) => println!("❌ Expected error: {}", e),
    }

    // Try to register a duplicate factory
    let new_container = SaDi::new().factory(|_| "first".to_string());
    match new_container.try_factory(|_| "duplicate".to_string()) {
        Ok(_) => println!("This shouldn't happen either"),
        Err(e) => println!("❌ Expected error: {}", e),
    }

    println!("\n🎉 Example completed successfully!");
    println!("\nKey takeaways:");
    println!("• Singletons (Config, Database) are created once and shared");
    println!("• Transients (Logger, UserService, etc.) are created fresh each time");
    println!("• Dependencies are automatically injected based on factory functions");
    println!("• Error handling provides clear feedback for missing or duplicate services");
    println!("• The container manages the entire object graph automatically");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_setup() {
        let container = SaDi::new()
            .factory_singleton(|_| ConfigService::new())
            .factory_singleton(|di: &SaDi| {
                DatabaseService::new(di.get_singleton::<ConfigService>())
            })
            .factory(|di: &SaDi| {
                LoggerService::new(
                    di.get_singleton::<DatabaseService>(),
                    di.get_singleton::<ConfigService>(),
                )
            });

        let logger = container.get::<LoggerService>();
        logger.log("TEST", "Container setup works!");
    }

    #[test]
    fn test_singleton_sharing() {
        let container = SaDi::new().factory_singleton(|_| ConfigService::new());

        let config1 = container.get_singleton::<ConfigService>();
        let config2 = container.get_singleton::<ConfigService>();

        // Same instance
        assert_eq!(Rc::as_ptr(&config1), Rc::as_ptr(&config2));
    }

    #[test]
    fn test_transient_behavior() {
        let container = SaDi::new()
            .factory_singleton(|_| ConfigService::new())
            .factory_singleton(|di: &SaDi| {
                DatabaseService::new(di.get_singleton::<ConfigService>())
            })
            .factory(|di: &SaDi| {
                LoggerService::new(
                    di.get_singleton::<DatabaseService>(),
                    di.get_singleton::<ConfigService>(),
                )
            });

        let logger1 = container.get::<LoggerService>();
        let logger2 = container.get::<LoggerService>();

        // Different instances
        assert_ne!(&logger1 as *const _, &logger2 as *const _);

        // But same database
        assert_eq!(Rc::as_ptr(&logger1.db), Rc::as_ptr(&logger2.db));
    }

    #[test]
    fn test_user_registration() {
        let container = SaDi::new()
            .factory_singleton(|_| ConfigService::new())
            .factory_singleton(|di: &SaDi| {
                DatabaseService::new(di.get_singleton::<ConfigService>())
            })
            .factory(|di: &SaDi| {
                LoggerService::new(
                    di.get_singleton::<DatabaseService>(),
                    di.get_singleton::<ConfigService>(),
                )
            })
            .factory(|di: &SaDi| {
                UserService::new(
                    di.get_singleton::<DatabaseService>(),
                    di.get::<LoggerService>(),
                )
            })
            .factory(|di: &SaDi| {
                EmailService::new(
                    di.get_singleton::<ConfigService>(),
                    di.get::<LoggerService>(),
                )
            })
            .factory(|di: &SaDi| {
                ApplicationService::new(
                    di.get::<UserService>(),
                    di.get::<EmailService>(),
                    di.get_singleton::<ConfigService>(),
                )
            });

        let app = container.get::<ApplicationService>();
        let result = app.register_user("Test User", "test@example.com");

        assert!(result.is_ok());
        assert!(result.unwrap().contains("Test User"));
    }
}
