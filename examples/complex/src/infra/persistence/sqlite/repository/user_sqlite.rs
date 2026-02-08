use std::sync::Arc;

use crate::core::domain::user::{User, UserRepository};
use crate::infra::persistence::sqlite::SqliteClient;

pub struct UserSqliteRepository {
    sqlite_client: Arc<SqliteClient>,
}

impl UserSqliteRepository {
    pub fn new(sqlite_client: Arc<SqliteClient>) -> Self {
        Self { sqlite_client }
    }
}

#[async_trait::async_trait]
impl UserRepository for UserSqliteRepository {
    async fn get_all(&self) -> Result<Vec<User>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query = "SELECT id, name, email FROM users";
        let mut statement = connection
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let mut users = Vec::new();
        while let Ok(sqlite::State::Row) = statement.next() {
            users.push(User {
                id: statement.read::<i64, _>(0).map_err(|e| e.to_string())? as u32,
                name: statement.read::<String, _>(1).map_err(|e| e.to_string())?,
                email: statement.read::<String, _>(2).map_err(|e| e.to_string())?,
            });
        }

        Ok(users)
    }

    async fn get_by_id(&self, id: u32) -> Result<Option<User>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query = "SELECT id, name, email FROM users WHERE id = ?";
        let mut statement = connection
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        statement
            .bind((1, id as i64))
            .map_err(|e| format!("Failed to bind parameter: {}", e))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            Ok(Some(User {
                id: statement.read::<i64, _>(0).map_err(|e| e.to_string())? as u32,
                name: statement.read::<String, _>(1).map_err(|e| e.to_string())?,
                email: statement.read::<String, _>(2).map_err(|e| e.to_string())?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn create(&self, name: String, email: String) -> Result<User, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query = "INSERT INTO users (name, email) VALUES (?, ?)";
        let mut statement = connection
            .prepare(query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        statement
            .bind((1, name.as_str()))
            .map_err(|e| format!("Failed to bind name: {}", e))?;
        statement
            .bind((2, email.as_str()))
            .map_err(|e| format!("Failed to bind email: {}", e))?;

        statement
            .next()
            .map_err(|e| format!("Failed to execute insert: {}", e))?;

        let query = "SELECT last_insert_rowid()";
        let mut stmt = connection
            .prepare(query)
            .map_err(|e| format!("Failed to get last insert id: {}", e))?;
        stmt.next().map_err(|e| format!("Failed to get last insert id: {}", e))?;
        let id = stmt.read::<i64, _>(0).map_err(|e| e.to_string())? as u32;

        Ok(User { id, name, email })
    }

    async fn delete(&self, id: u32) -> Result<bool, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let query = "DELETE FROM users WHERE id = ?";
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
