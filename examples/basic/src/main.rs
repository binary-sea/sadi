use sadi::{Container, Error, Shared, bind, container};

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
    fn new(config: Shared<ConfigService>) -> Self {
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
    db: Shared<DatabaseService>,
    config: Shared<ConfigService>,
}

impl LoggerService {
    fn new(db: Shared<DatabaseService>, config: Shared<ConfigService>) -> Self {
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
    db: Shared<DatabaseService>,
    logger: Shared<LoggerService>,
}

impl UserService {
    fn new(db: Shared<DatabaseService>, logger: Shared<LoggerService>) -> Self {
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
    config: Shared<ConfigService>,
    logger: Shared<LoggerService>,
}

impl EmailService {
    fn new(config: Shared<ConfigService>, logger: Shared<LoggerService>) -> Self {
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
    user_service: Shared<UserService>,
    email_service: Shared<EmailService>,
    config: Shared<ConfigService>,
}

impl ApplicationService {
    fn new(
        user_service: Shared<UserService>,
        email_service: Shared<EmailService>,
        config: Shared<ConfigService>,
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

    let container = container! {
        bind(singleton ConfigService => |_| ConfigService::new())
        bind(singleton DatabaseService => |c| DatabaseService::new(c.resolve::<ConfigService>().unwrap()))
        bind(LoggerService => |c| LoggerService::new(c.resolve::<DatabaseService>().unwrap(), c.resolve::<ConfigService>().unwrap()))
        bind(UserService => |c| UserService::new(c.resolve::<DatabaseService>().unwrap(), c.resolve::<LoggerService>().unwrap()))
        bind(EmailService => |c| EmailService::new(c.resolve::<ConfigService>().unwrap(), c.resolve::<LoggerService>().unwrap()))
        bind(ApplicationService => |c| ApplicationService::new(c.resolve::<UserService>().unwrap(), c.resolve::<EmailService>().unwrap(), c.resolve::<ConfigService>().unwrap()))
    };

    println!("✅ Container configured successfully!\n");

    // Demonstrate singleton behavior
    println!("--- Singleton Behavior ---");
    let config1 = container.resolve::<ConfigService>().unwrap();
    let config2 = container.resolve::<ConfigService>().unwrap();

    println!("📋 Config1: {}", config1.get_info());
    println!("📋 Config2: {}", config2.get_info());
    println!("🔄 Same instance? {}", Shared::ptr_eq(&config1, &config2));

    // Use the application
    println!("\n--- Application Usage ---");
    let app = container.resolve::<ApplicationService>().unwrap();

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
    let user_service1 = container.resolve::<UserService>().unwrap();
    let user_service2 = container.resolve::<UserService>().unwrap();

    println!(
        "🔄 Different UserService instances? {}",
        !Shared::ptr_eq(&user_service1, &user_service2)
    );

    // But they share the same database singleton
    println!(
        "🔄 Same database instance? {}",
        Shared::ptr_eq(&user_service1.db, &user_service2.db)
    );

    // Show some queries
    println!("\n--- Database Queries ---");
    println!("📊 {}", user_service1.get_user(1));
    println!("📊 {}", user_service2.get_user(2));

    // Demonstrate error handling
    println!("\n--- Error Handling ---");

    // Try to get a service that wasn't registered
    match container.resolve::<String>() {
        Ok(_) => println!("This shouldn't happen"),
        Err(e) => println!("❌ Expected error: {}", e),
    }

    // Try to register a duplicate factory
    let new_container = Container::new();
    new_container
        .bind_concrete::<String, String, _>(|_| "first".to_string())
        .unwrap();
    match new_container.bind_concrete::<String, String, _>(|_| "duplicate".to_string()) {
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
        let c = Container::new();
        c.bind_concrete_singleton::<ConfigService, ConfigService, _>(|_| ConfigService::new())
            .unwrap();
        c.bind_concrete_singleton::<DatabaseService, DatabaseService, _>(|c| {
            DatabaseService::new(c.resolve::<ConfigService>().unwrap())
        })
        .unwrap();
        c.bind_concrete::<LoggerService, LoggerService, _>(|c| {
            LoggerService::new(
                c.resolve::<DatabaseService>().unwrap(),
                c.resolve::<ConfigService>().unwrap(),
            )
        })
        .unwrap();

        let logger = c.resolve::<LoggerService>().unwrap();
        logger.log("TEST", "Container setup works!");
    }

    #[test]
    fn test_singleton_sharing() {
        let c = Container::new();
        c.bind_concrete_singleton::<ConfigService, ConfigService, _>(|_| ConfigService::new())
            .unwrap();

        let config1 = c.resolve::<ConfigService>().unwrap();
        let config2 = c.resolve::<ConfigService>().unwrap();

        // Same instance
        assert!(Shared::ptr_eq(&config1, &config2));
    }

    #[test]
    fn test_transient_behavior() {
        let c = Container::new();
        c.bind_concrete_singleton::<ConfigService, ConfigService, _>(|_| ConfigService::new())
            .unwrap();
        c.bind_concrete_singleton::<DatabaseService, DatabaseService, _>(|c| {
            DatabaseService::new(c.resolve::<ConfigService>().unwrap())
        })
        .unwrap();
        c.bind_concrete::<LoggerService, LoggerService, _>(|c| {
            LoggerService::new(
                c.resolve::<DatabaseService>().unwrap(),
                c.resolve::<ConfigService>().unwrap(),
            )
        })
        .unwrap();

        let logger1 = c.resolve::<LoggerService>().unwrap();
        let logger2 = c.resolve::<LoggerService>().unwrap();

        // Different instances
        assert!(!Shared::ptr_eq(&logger1, &logger2));

        // But same database
        assert!(Shared::ptr_eq(&logger1.db, &logger2.db));
    }

    #[test]
    fn test_user_registration() {
        let c = Container::new();
        c.bind_concrete_singleton::<ConfigService, ConfigService, _>(|_| ConfigService::new())
            .unwrap();
        c.bind_concrete_singleton::<DatabaseService, DatabaseService, _>(|c| {
            DatabaseService::new(c.resolve::<ConfigService>().unwrap())
        })
        .unwrap();
        c.bind_concrete::<LoggerService, LoggerService, _>(|c| {
            LoggerService::new(
                c.resolve::<DatabaseService>().unwrap(),
                c.resolve::<ConfigService>().unwrap(),
            )
        })
        .unwrap();
        c.bind_concrete::<UserService, UserService, _>(|c| {
            UserService::new(
                c.resolve::<DatabaseService>().unwrap(),
                c.resolve::<LoggerService>().unwrap(),
            )
        })
        .unwrap();
        c.bind_concrete::<EmailService, EmailService, _>(|c| {
            EmailService::new(
                c.resolve::<ConfigService>().unwrap(),
                c.resolve::<LoggerService>().unwrap(),
            )
        })
        .unwrap();
        c.bind_concrete::<ApplicationService, ApplicationService, _>(|c| {
            ApplicationService::new(
                c.resolve::<UserService>().unwrap(),
                c.resolve::<EmailService>().unwrap(),
                c.resolve::<ConfigService>().unwrap(),
            )
        })
        .unwrap();

        let app = c.resolve::<ApplicationService>().unwrap();
        let result = app.register_user("Test User", "test@example.com");

        assert!(result.is_ok());
        assert!(result.unwrap().contains("Test User"));
    }
}
