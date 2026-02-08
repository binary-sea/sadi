use std::sync::Mutex;

pub struct SqliteClient {
    migrated: bool,
    connection: Mutex<sqlite::Connection>,
}

impl std::fmt::Debug for SqliteClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteClient")
            .field("migrated", &self.migrated)
            .field("connection", &"<Mutex<sqlite::Connection>>")
            .finish()
    }
}

impl std::fmt::Display for SqliteClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteClient")
            .field("migrated", &self.migrated)
            .field("connection", &"<Mutex<sqlite::Connection>>")
            .finish()
    }
}

impl SqliteClient {
    pub fn new() -> Result<Self, String> {
        let connection = sqlite::open(":memory:").map_err(|e| e.to_string())?;
        let mut client = Self {
            migrated: false,
            connection: Mutex::new(connection),
        };
        client.run_migrations()?;
        Ok(client)
    }

    pub fn run_migrations(&mut self) -> Result<(), String> {
        if self.migrated {
            return Ok(());
        }

        let connection = self
            .connection
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        // Create users table
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    email TEXT NOT NULL
                )",
            )
            .map_err(|e| format!("Failed to create users table: {}", e))?;

        // Create todos table
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS todos (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    description TEXT NOT NULL,
                    completed INTEGER NOT NULL DEFAULT 0,
                    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
                )",
            )
            .map_err(|e| format!("Failed to create todos table: {}", e))?;

        self.migrated = true;
        Ok(())
    }

    pub fn is_migrated(&self) -> bool {
        self.migrated
    }

    pub fn connection(&self) -> &Mutex<sqlite::Connection> {
        &self.connection
    }
}
