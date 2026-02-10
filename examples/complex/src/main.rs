use crate::core::application::use_case::{
    todo::{CreateTodoUseCase, GetAllTodoUseCase, UpdateStatusTodoUseCase, DeleteTodoUseCase},
    user::{CreateUserUseCase, GetAllUserUseCase, GetByIdUserUseCase, DeleteUserUseCase},
};
use crate::infra::persistence::sqlite::SqliteClient;

pub mod core;
pub mod infra;

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt::init();
    
    println!("=== Complex Example: DI with SQLite Repositories ===\n");

    // Build the application with dependency injection
    let app = infra::di::build().expect("Failed to build application");
    println!("✓ Application built successfully\n");

    // Verify SqliteClient was initialized
    let sqlite_client = app.injector().try_resolve::<SqliteClient>()
        .map_err(|e| format!("Failed to resolve SqliteClient: {:?}", e))?;
    println!("✓ SqliteClient initialized: {}\n", sqlite_client);

    // === User Operations ===
    println!("--- User Operations ---");

    // Create users
    let create_user = app.injector().try_resolve::<CreateUserUseCase>()
        .map_err(|e| format!("Failed to resolve CreateUserUseCase: {:?}", e))?;
    
    let user1 = create_user.execute("Alice".to_string(), "alice@example.com".to_string()).await?;
    println!("✓ Created user: {:?}", user1);

    let user2 = create_user.execute("Bob".to_string(), "bob@example.com".to_string()).await?;
    println!("✓ Created user: {:?}", user2);

    let user3 = create_user.execute("Charlie".to_string(), "charlie@example.com".to_string()).await?;
    println!("✓ Created user: {:?}\n", user3);

    // Get all users
    let get_all_users = app.injector().try_resolve::<GetAllUserUseCase>()
        .map_err(|e| format!("Failed to resolve GetAllUserUseCase: {:?}", e))?;
    let users = get_all_users.execute().await?;
    println!("✓ All users ({}): {:?}\n", users.len(), users);

    // Get user by ID
    let get_user_by_id = app.injector().try_resolve::<GetByIdUserUseCase>()
        .map_err(|e| format!("Failed to resolve GetByIdUserUseCase: {:?}", e))?;
    let found_user = get_user_by_id.execute(user1.id).await?;
    println!("✓ Found user by ID {}: {:?}\n", user1.id, found_user);

    // === Todo Operations ===
    println!("--- Todo Operations ---");

    // Create todos
    let create_todo = app.injector().try_resolve::<CreateTodoUseCase>()
        .map_err(|e| format!("Failed to resolve CreateTodoUseCase: {:?}", e))?;
    
    let todo1 = create_todo.execute(
        user1.id,
        "Buy groceries".to_string(),
        "Milk, eggs, bread".to_string(),
    ).await?;
    println!("✓ Created todo: {:?}", todo1);

    let todo2 = create_todo.execute(
        user1.id,
        "Write documentation".to_string(),
        "Document the new API endpoints".to_string(),
    ).await?;
    println!("✓ Created todo: {:?}", todo2);

    let todo3 = create_todo.execute(
        user2.id,
        "Review PRs".to_string(),
        "Review pending pull requests".to_string(),
    ).await?;
    println!("✓ Created todo: {:?}\n", todo3);

    // Get all todos
    let get_all_todos = app.injector().try_resolve::<GetAllTodoUseCase>()
        .map_err(|e| format!("Failed to resolve GetAllTodoUseCase: {:?}", e))?;
    let todos = get_all_todos.execute().await?;
    println!("✓ All todos ({}): {:?}\n", todos.len(), todos);

    // Update todo status
    let update_status = app.injector().try_resolve::<UpdateStatusTodoUseCase>()
        .map_err(|e| format!("Failed to resolve UpdateStatusTodoUseCase: {:?}", e))?;
    let updated_todo = update_status.execute(todo1.id, true).await?;
    println!("✓ Updated todo status: {:?}\n", updated_todo);

    // Get all todos after update
    let todos = get_all_todos.execute().await?;
    println!("✓ All todos after update: {:?}\n", todos);

    // Delete a todo
    let delete_todo = app.injector().try_resolve::<DeleteTodoUseCase>()
        .map_err(|e| format!("Failed to resolve DeleteTodoUseCase: {:?}", e))?;
    let deleted = delete_todo.execute(todo2.id).await?;
    println!("✓ Deleted todo {}: {}\n", todo2.id, deleted);

    // Get all todos after deletion
    let todos = get_all_todos.execute().await?;
    println!("✓ All todos after deletion ({}): {:?}\n", todos.len(), todos);

    // Delete a user
    let delete_user = app.injector().try_resolve::<DeleteUserUseCase>()
        .map_err(|e| format!("Failed to resolve DeleteUserUseCase: {:?}", e))?;
    let deleted = delete_user.execute(user3.id).await?;
    println!("✓ Deleted user {}: {}\n", user3.id, deleted);

    // Get all users after deletion
    let users = get_all_users.execute().await?;
    println!("✓ All users after deletion ({}): {:?}\n", users.len(), users);

    println!("=== Example completed successfully! ===");
    Ok(())
}
