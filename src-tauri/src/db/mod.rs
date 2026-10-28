//! Database access: connection management, pragmas, and per-user data paths.

pub mod migrate;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::domain::error::{AppError, AppResult};

/// Resolved on-disk locations for user data.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Root directory for all user data (`%APPDATA%\<identifier>`).
    pub root: PathBuf,
    /// The SQLite database file.
    pub database: PathBuf,
    /// Directory holding the logo / brand assets.
    pub brand: PathBuf,
    /// Default backup destination.
    pub backups: PathBuf,
}

impl AppPaths {
    /// Build the per-user paths. User data never lives next to the executable
    /// (PRD §4), so an installer upgrade can never replace it.
    pub fn resolve(identifier: &str) -> AppResult<Self> {
        let base = dirs::data_dir()
            .or_else(dirs::home_dir)
            .ok_or_else(|| AppError::internal("تعذّر تحديد مجلد بيانات المستخدم"))?;
        let root = base.join(identifier);
        let paths = Self {
            database: root.join("data").join("invoice.db"),
            brand: root.join("data").join("brand"),
            backups: root.join("backups"),
            root,
        };
        paths.ensure_dirs()?;
        Ok(paths)
    }

    pub fn ensure_dirs(&self) -> AppResult<()> {
        for dir in [
            self.root.clone(),
            self.root.join("data"),
            self.brand.clone(),
            self.backups.clone(),
        ] {
            std::fs::create_dir_all(&dir).map_err(|e| {
                AppError::storage(format!("تعذّر إنشاء المجلد {}: {e}", dir.display()))
            })?;
        }
        Ok(())
    }

    /// Path of the stored brand logo SVG.
    pub fn logo_file(&self) -> PathBuf {
        self.brand.join("logo.svg")
    }
}

/// Open the database with the pragmas the application relies on.
pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::storage(format!("تعذّر إنشاء مجلد قاعدة البيانات: {e}")))?;
    }
    let conn = Connection::open(path)
        .map_err(|e| AppError::storage(format!("تعذّر فتح قاعدة البيانات: {e}")))?;

    // WAL keeps reads fast while a write is in flight; the Online Backup API
    // then produces a consistent snapshot (PRD §9).
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;

    migrate::run(&conn)?;
    Ok(conn)
}

/// Open an in-memory database with the same pragmas, for tests and fixtures.
pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate::run(&conn)?;
    Ok(conn)
}

/// Current UTC timestamp in RFC 3339, used for all `*_at` columns.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_database_has_schema() {
        let conn = open_in_memory().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(count >= 10);
    }

    #[test]
    fn paths_resolve_under_data_dir() {
        let paths = AppPaths::resolve("com.ua.invoice.test").unwrap();
        assert!(paths.database.ends_with("data/invoice.db"));
        assert!(paths.brand.ends_with("data/brand"));
        assert!(paths.backups.ends_with("backups"));
    }
}
