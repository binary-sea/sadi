use std::sync::Arc;

use crate::core::domain::todo::{Todo, TodoRepository};

pub struct CreateTodoUseCase {
    todo_repository: Arc<dyn TodoRepository>,
}

impl CreateTodoUseCase {
    pub fn new(todo_repository: Arc<dyn TodoRepository>) -> Self {
        Self { todo_repository }
    }

    pub async fn execute(&self, user_id: u32, title: String, description: String) -> Result<Todo, String> {
        // Business logic can be added here (e.g., validation, logging, etc.)
        // For simplicity, we directly call the repository to create the todo.
        self.todo_repository.create(user_id, title, description).await
    }
}
