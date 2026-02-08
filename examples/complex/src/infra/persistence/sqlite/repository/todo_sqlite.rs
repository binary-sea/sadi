use std::sync::Arc;

use crate::core::domain::todo::{Todo, TodoRepository};
use crate::infra::persistence::sqlite::SqliteClient;

pub struct TodoSqliteRepository {
    sqlite_client: Arc<SqliteClient>,
}

impl TodoSqliteRepository {
    pub fn new(sqlite_client: Arc<SqliteClient>) -> Self {
        Self { sqlite_client }
    }
}

#[async_trait::async_trait]
impl TodoRepository for TodoSqliteRepository {
    async fn get_all(&self) -> Result<Vec<Todo>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query = "SELECT id, title, description, completed FROM todos";
        let mut statement = connection
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let mut todos = Vec::new();
        while let Ok(sqlite::State::Row) = statement.next() {
            todos.push(Todo {
                id: statement.read::<i64, _>(0).map_err(|e| e.to_string())? as u32,
                title: statement.read::<String, _>(1).map_err(|e| e.to_string())?,
                description: statement.read::<String, _>(2).map_err(|e| e.to_string())?,
                completed: statement.read::<i64, _>(3).map_err(|e| e.to_string())? != 0,
            });
        }

        Ok(todos)
    }

    async fn get_by_id(&self, id: u32) -> Result<Option<Todo>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query = "SELECT id, title, description, completed FROM todos WHERE id = ?";
        let mut statement = connection
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        statement
            .bind((1, id as i64))
            .map_err(|e| format!("Failed to bind parameter: {}", e))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            Ok(Some(Todo {
                id: statement.read::<i64, _>(0).map_err(|e| e.to_string())? as u32,
                title: statement.read::<String, _>(1).map_err(|e| e.to_string())?,
                description: statement.read::<String, _>(2).map_err(|e| e.to_string())?,
                completed: statement.read::<i64, _>(3).map_err(|e| e.to_string())? != 0,
            }))
        } else {
            Ok(None)
        }
    }

    async fn create(
        &self,
        user_id: u32,
        title: String,
        description: String,
    ) -> Result<Todo, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query =
            "INSERT INTO todos (user_id, title, description, completed) VALUES (?, ?, ?, 0)";
        let mut statement = connection
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        statement
            .bind((1, user_id as i64))
            .map_err(|e| format!("Failed to bind user_id: {}", e))?;
        statement
            .bind((2, title.as_str()))
            .map_err(|e| format!("Failed to bind title: {}", e))?;
        statement
            .bind((3, description.as_str()))
            .map_err(|e| format!("Failed to bind description: {}", e))?;

        statement
            .next()
            .map_err(|e| format!("Failed to execute insert: {}", e))?;

        let query = "SELECT last_insert_rowid()";
        let mut stmt = connection
            .prepare(query)
            .map_err(|e| format!("Failed to get last insert id: {}", e))?;
        stmt.next().map_err(|e| format!("Failed to get last insert id: {}", e))?;
        let id = stmt.read::<i64, _>(0).map_err(|e| e.to_string())? as u32;

        Ok(Todo {
            id,
            title,
            description,
            completed: false,
        })
    }

    async fn update_status(&self, id: u32, completed: bool) -> Result<Option<Todo>, String> {
        let updated = {
            let connection = self
                .sqlite_client
                .connection()
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let query = "UPDATE todos SET completed = ? WHERE id = ?";
            let mut statement = connection
                .prepare(query)
                .map_err(|e| format!("Failed to prepare query: {}", e))?;

            statement
                .bind((1, if completed { 1i64 } else { 0i64 }))
                .map_err(|e| format!("Failed to bind completed: {}", e))?;
            statement
                .bind((2, id as i64))
                .map_err(|e| format!("Failed to bind id: {}", e))?;

            statement
                .next()
                .map_err(|e| format!("Failed to execute update: {}", e))?;

            connection.change_count() > 0
        };

        if updated {
            self.get_by_id(id).await
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, id: u32) -> Result<bool, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query = "DELETE FROM todos WHERE id = ?";
        let mut statement = connection
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        statement
            .bind((1, id as i64))
            .map_err(|e| format!("Failed to bind id: {}", e))?;

        statement
            .next()
            .map_err(|e| format!("Failed to execute delete: {}", e))?;

        Ok(connection.change_count() > 0)
    }
}
