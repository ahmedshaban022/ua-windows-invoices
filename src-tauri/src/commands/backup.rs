//! Backup and restore commands (FR-09).

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::db;
use crate::domain::error::{AppError, AppResult};
use crate::services::backup::{self, BackupInfo, BackupValidation};
use crate::state::AppState;

/// A backup file found in the backup folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

/// Create a backup archive.
///
/// When `destination` is empty the configured backup folder (or the default
/// one) is used and the file name carries a timestamp.
#[tauri::command]
pub fn create_backup(app: AppHandle, destination: Option<String>) -> AppResult<BackupInfo> {
    let state = app.state::<AppState>();
    let settings = state.settings()?;

    let target = match destination.filter(|value| !value.trim().is_empty()) {
        Some(path) => PathBuf::from(path),
        None => {
            let folder = if settings.backup_location.trim().is_empty() {
                state.paths.backups.clone()
            } else {
                PathBuf::from(settings.backup_location.trim())
            };
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            folder.join(format!("invoice-backup-{stamp}.zip"))
        }
    };

    let info =
        state.with_conn(|conn| backup::create_backup(conn, &target, Some(&state.paths.brand)))?;

    // Record the attempt in the history table.
    let _ = state.with_conn(|conn| -> AppResult<()> {
        conn.execute(
            "INSERT INTO backup_history(destination, file_path, sha256, size_bytes, result, created_at)
             VALUES (?1, ?2, ?3, ?4, 'success', ?5)",
            params![
                target.to_string_lossy().to_string(),
                info.file_path,
                info.sha256,
                info.size_bytes as i64,
                crate::db::now()
            ],
        )?;
        Ok(())
    });

    Ok(info)
}

/// Validate a backup file without restoring it.
#[tauri::command]
pub fn validate_backup(path: String) -> AppResult<BackupValidation> {
    backup::validate_backup(Path::new(&path))
}

/// Restore the database (and the brand assets) from a backup file.
///
/// The live database is closed first, a safety backup is taken, the archive is
/// validated and integrity-checked, and only then is the file swapped. On any
/// failure the existing database is kept.
#[tauri::command]
pub fn restore_backup(app: AppHandle, path: String) -> AppResult<BackupInfo> {
    let state = app.state::<AppState>();
    let database = state.paths.database.clone();
    let brand = state.paths.brand.clone();
    let safety_dir = state.paths.backups.clone();

    // Close the live connection so the file can be replaced safely.
    {
        let mut guard = state.conn.write();
        let placeholder = Connection::open_in_memory()
            .map_err(|e| AppError::storage(format!("تعذّر تجهيز قاعدة البيانات: {e}")))?;
        let previous = std::mem::replace(
            &mut *guard,
            std::sync::Arc::new(parking_lot::Mutex::new(placeholder)),
        );
        // Dropping the old handle closes the file on Windows.
        drop(previous);
    }

    let result =
        backup::restore_backup(&database, Path::new(&path), Some(&brand), Some(&safety_dir));

    // Reopen the database whatever happened; on failure this simply reopens the
    // untouched original file.
    match db::open(&database) {
        Ok(conn) => state.replace_conn(conn),
        Err(error) => {
            return Err(AppError::storage(format!(
                "تعذّر إعادة فتح قاعدة البيانات بعد الاستعادة: {error}"
            )))
        }
    }

    let info = result?;
    let _ = state.with_conn(|conn| -> AppResult<()> {
        conn.execute(
            "INSERT INTO backup_history(destination, file_path, sha256, size_bytes, result, created_at)
             VALUES (?1, ?2, ?3, ?4, 'restore', ?5)",
            params![
                path.clone(),
                info.file_path,
                info.sha256,
                info.size_bytes as i64,
                crate::db::now()
            ],
        )?;
        Ok(())
    });
    Ok(info)
}

/// List the backup files in the configured backup folder.
#[tauri::command]
pub fn list_backups(app: AppHandle) -> AppResult<Vec<BackupFile>> {
    let state = app.state::<AppState>();
    let settings = state.settings()?;
    let folder = if settings.backup_location.trim().is_empty() {
        state.paths.backups.clone()
    } else {
        PathBuf::from(settings.backup_location.trim())
    };

    let entries = match std::fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()),
    };

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let created_at = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|duration| chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0))
            .map(|date| date.to_rfc3339())
            .unwrap_or_default();
        files.push(BackupFile {
            file_name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            file_path: path.to_string_lossy().to_string(),
            size_bytes: metadata.len(),
            created_at,
        });
    }
    files.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(files)
}

/// Remove old backups, keeping the most recent `retention` files.
#[tauri::command]
pub fn prune_backups(app: AppHandle, retention: Option<usize>) -> AppResult<usize> {
    let state = app.state::<AppState>();
    let settings = state.settings()?;
    let retention = retention.unwrap_or(settings.auto_backup_retention.max(1) as usize);
    let folder = if settings.backup_location.trim().is_empty() {
        state.paths.backups.clone()
    } else {
        PathBuf::from(settings.backup_location.trim())
    };
    backup::prune_backups(&folder, retention)
}

/// The last successful backup timestamp, shown in Settings.
#[tauri::command]
pub fn last_backup(app: AppHandle) -> AppResult<Option<String>> {
    let state = app.state::<AppState>();
    state.with_conn(|conn| {
        let timestamp: Option<String> = conn
            .query_row(
                "SELECT created_at FROM backup_history WHERE result = 'success'
                 ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();
        Ok(timestamp)
    })
}
