use std::{fs, path::PathBuf};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::date::local_date;

pub(crate) struct Database {
    path: PathBuf,
}

impl Database {
    pub(crate) fn open() -> Result<Self, String> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| "Windows Local AppData is not available.".to_owned())?;
        let directory = PathBuf::from(local_app_data).join("Aster");
        fs::create_dir_all(&directory).map_err(|error| format!("Create data folder: {error}"))?;
        let path = directory.join("aster.sqlite3");
        let db = Self { path };
        let connection = db.connection()?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Read schema version: {error}"))?;
        if version > 1 {
            return Err(format!(
                "Database schema version {version} is newer than this app."
            ));
        }
        if version == 0 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                CREATE TABLE financial_years (
                    start_year INTEGER PRIMARY KEY,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE users (
                    id INTEGER PRIMARY KEY,
                    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
                    password_hash TEXT NOT NULL,
                    role TEXT NOT NULL DEFAULT 'admin',
                    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                PRAGMA user_version = 1;
                COMMIT;",
                )
                .map_err(|error| format!("Initialize SQLite schema: {error}"))?;
        }
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|error| format!("Check database integrity: {error}"))?;
        if integrity != "ok" {
            return Err(format!("SQLite integrity check failed: {integrity}"));
        }
        let (year, month, _) = local_date();
        let current_start = if month >= 4 { year } else { year - 1 };
        connection
            .execute(
                "INSERT OR IGNORE INTO financial_years (start_year) VALUES (?1)",
                [current_start],
            )
            .map_err(|error| format!("Create current financial year: {error}"))?;
        Ok(db)
    }

    pub(crate) fn connection(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path)
            .map_err(|error| format!("Open database {}: {error}", self.path.display()))?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| format!("Enable database foreign keys: {error}"))?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|error| format!("Configure SQLite journaling: {error}"))?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(|error| format!("Configure SQLite synchronization: {error}"))?;
        Ok(connection)
    }

    pub(crate) fn financial_years(&self) -> Result<Vec<i32>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT start_year FROM financial_years ORDER BY start_year DESC")
            .map_err(|error| format!("Read financial years: {error}"))?;
        let years = statement
            .query_map([], |row| row.get(0))
            .map_err(|error| format!("Read financial years: {error}"))?
            .collect::<Result<Vec<i32>, _>>()
            .map_err(|error| format!("Read financial years: {error}"))?;
        Ok(years)
    }

    pub(crate) fn create_financial_year(&self, start_year: i32) -> Result<(), String> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO financial_years (start_year) VALUES (?1)",
                [start_year],
            )
            .map_err(|error| {
                format!(
                    "Create financial year {start_year}–{}: {error}",
                    start_year + 1
                )
            })?;
        Ok(())
    }

    pub(crate) fn has_users(&self) -> Result<bool, String> {
        let connection = self.connection()?;
        connection
            .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
            .map_err(|error| format!("Check administrator setup: {error}"))
    }

    pub(crate) fn create_admin(&self, username: &str, password: &str) -> Result<(), String> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|error| format!("Hash password: {error}"))?
            .to_string();
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Start administrator setup: {error}"))?;
        let already_set_up: bool = transaction
            .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
            .map_err(|error| format!("Check administrator setup: {error}"))?;
        if already_set_up {
            return Err("Administrator setup has already been completed.".into());
        }
        transaction
            .execute(
                "INSERT INTO users (username, password_hash, role, active) VALUES (?1, ?2, 'admin', 1)",
                params![username, password_hash],
            )
            .map_err(|error| {
                if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                    "That user name already exists.".to_owned()
                } else {
                    format!("Save administrator: {error}")
                }
            })?;
        transaction
            .commit()
            .map_err(|error| format!("Finish administrator setup: {error}"))?;
        Ok(())
    }

    pub(crate) fn verify_user(&self, username: &str, password: &str) -> Result<bool, String> {
        let connection = self.connection()?;
        let password_hash: Option<String> = connection
            .query_row(
                "SELECT password_hash FROM users WHERE username = ?1 AND active = 1",
                [username],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("Read user credentials: {error}"))?;
        let Some(password_hash) = password_hash else {
            return Ok(false);
        };
        let parsed = PasswordHash::new(&password_hash)
            .map_err(|error| format!("Stored password hash is invalid: {error}"))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    pub(crate) fn change_password(
        &self,
        username: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        if !self.verify_user(username, current_password)? {
            return Err("Your current password is incorrect.".into());
        }
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|error| format!("Hash password: {error}"))?
            .to_string();
        let connection = self.connection()?;
        connection
            .execute(
                "UPDATE users SET password_hash = ?1 WHERE username = ?2 AND active = 1",
                params![password_hash, username],
            )
            .map_err(|error| format!("Update password: {error}"))?;
        Ok(())
    }
}
