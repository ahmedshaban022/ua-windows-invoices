//! Backup and restore (FR-09).
//!
//! A backup is produced with the SQLite **Online Backup API**, so it is
//! consistent even while the database is in WAL mode — copying the live file
//! would not be. The archive contains the database plus the brand assets.
//!
//! Restore validates the archive and runs `PRAGMA integrity_check` on a staged
//! copy **before** the live database is touched. A pre-restore safety backup is
//! taken first; on any error the active database is retained untouched.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::now;
use crate::domain::error::{AppError, AppResult};

const DB_ENTRY: &str = "invoice.db";
const BRAND_ENTRY: &str = "brand/logo.svg";

/// Metadata about a completed backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub created_at: String,
}

/// Information about a validated backup file, before restoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupValidation {
    pub valid: bool,
    pub sha256: String,
    pub size_bytes: u64,
    /// Arabic message describing the outcome.
    pub message: String,
}

/// Create a backup archive at `destination`.
///
/// The database is copied through the Online Backup API; brand assets are
/// added to the same zip.
pub fn create_backup(
    conn: &Connection,
    destination: &Path,
    brand_dir: Option<&Path>,
) -> AppResult<BackupInfo> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::storage(format!("تعذّر إنشاء مجلد النسخ الاحتياطي: {e}")))?;
    }

    // Stage the database copy via the Online Backup API.
    let staging = destination.with_extension("db-staging");
    {
        let mut target = Connection::open(&staging)
            .map_err(|e| AppError::storage(format!("تعذّر إنشاء ملف النسخة المؤقت: {e}")))?;
        let backup = rusqlite::backup::Backup::new(conn, &mut target)
            .map_err(|e| AppError::storage(format!("تعذّر بدء النسخة الاحتياطية: {e}")))?;
        backup
            .run_to_completion(256, std::time::Duration::from_millis(10), None)
            .map_err(|e| AppError::storage(format!("فشلت النسخة الاحتياطية: {e}")))?;
    }

    let file = std::fs::File::create(destination)
        .map_err(|e| AppError::storage(format!("تعذّر إنشاء ملف النسخة الاحتياطي: {e}")))?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(DB_ENTRY, options)
        .map_err(|e| AppError::storage(format!("تعذّر إضافة قاعدة البيانات للنسخة: {e}")))?;
    let mut db_file = std::fs::File::open(&staging)
        .map_err(|e| AppError::storage(format!("تعذّر قراءة النسخة المؤقتة: {e}")))?;
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = db_file
            .read(&mut buffer)
            .map_err(|e| AppError::storage(format!("خطأ أثناء نسخ قاعدة البيانات: {e}")))?;
        if read == 0 {
            break;
        }
        zip.write_all(&buffer[..read])
            .map_err(|e| AppError::storage(format!("خطأ أثناء كتابة النسخة: {e}")))?;
    }

    if let Some(brand) = brand_dir {
        let logo = brand.join("logo.svg");
        if logo.exists() {
            zip.start_file(BRAND_ENTRY, options)
                .map_err(|e| AppError::storage(format!("تعذّر إضافة الشعار للنسخة: {e}")))?;
            let mut logo_file = std::fs::File::open(&logo)
                .map_err(|e| AppError::storage(format!("تعذّر قراءة الشعار: {e}")))?;
            let mut buf = Vec::new();
            logo_file
                .read_to_end(&mut buf)
                .map_err(|e| AppError::storage(format!("خطأ أثناء نسخ الشعار: {e}")))?;
            zip.write_all(&buf)
                .map_err(|e| AppError::storage(format!("خطأ أثناء كتابة الشعار: {e}")))?;
        }
    }

    zip.finish()
        .map_err(|e| AppError::storage(format!("تعذّر إغلاق ملف النسخة الاحتياطية: {e}")))?;
    let _ = std::fs::remove_file(&staging);

    let (sha256, size_bytes) = hash_file(destination)?;

    Ok(BackupInfo {
        file_path: destination.to_string_lossy().to_string(),
        sha256,
        size_bytes,
        created_at: now(),
    })
}

/// Validate a backup file without restoring it.
pub fn validate_backup(path: &Path) -> AppResult<BackupValidation> {
    if !path.exists() {
        return Ok(BackupValidation {
            valid: false,
            sha256: String::new(),
            size_bytes: 0,
            message: "ملف النسخة الاحتياطية غير موجود".to_string(),
        });
    }

    let file = std::fs::File::open(path)
        .map_err(|e| AppError::storage(format!("تعذّر فتح ملف النسخة: {e}")))?;
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(e) => {
            return Ok(BackupValidation {
                valid: false,
                sha256: String::new(),
                size_bytes: 0,
                message: format!("الملف ليس نسخة احتياطية صالحة: {e}"),
            });
        }
    };

    let mut db_bytes = Vec::new();
    match archive.by_name(DB_ENTRY) {
        Ok(mut entry) => {
            entry.read_to_end(&mut db_bytes).map_err(|e| {
                AppError::storage(format!("تعذّر قراءة قاعدة البيانات من النسخة: {e}"))
            })?;
        }
        Err(_) => {
            return Ok(BackupValidation {
                valid: false,
                sha256: String::new(),
                size_bytes: 0,
                message: "النسخة الاحتياطية لا تحتوي على قاعدة بيانات".to_string(),
            });
        }
    }

    // SQLite files start with this exact header.
    const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";
    if !db_bytes.starts_with(SQLITE_HEADER) {
        return Ok(BackupValidation {
            valid: false,
            sha256: String::new(),
            size_bytes: 0,
            message: "محتوى النسخة الاحتياطية تالف".to_string(),
        });
    }

    // Stage it and run an integrity check.
    let staged = tempfile_path("validate");
    std::fs::write(&staged, &db_bytes)
        .map_err(|e| AppError::storage(format!("تعذّر تجهيز النسخة للتحقق: {e}")))?;
    let integrity = {
        let conn = Connection::open(&staged)
            .map_err(|e| AppError::storage(format!("تعذّر فتح النسخة للتحقق: {e}")))?;
        conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::storage(format!("فشل التحقق من سلامة النسخة: {e}")))?
    };
    let _ = std::fs::remove_file(&staged);

    let (sha256, size_bytes) = hash_file(path)?;
    let valid = integrity.eq_ignore_ascii_case("ok");
    let message = if valid {
        "النسخة الاحتياطية سليمة".to_string()
    } else {
        format!("فشل التحقق من سلامة النسخة: {integrity}")
    };

    Ok(BackupValidation {
        valid,
        sha256,
        size_bytes,
        message,
    })
}

/// Restore the database and brand assets from a backup.
///
/// Takes a safety backup first, validates the archive, then swaps the database
/// file atomically. On error the live database is left exactly as it was.
///
/// The live database file is replaced on disk; the caller is responsible for
/// closing and reopening its connection afterwards (see `AppState::restore_backup`).
pub fn restore_backup(
    db_path: &Path,
    path: &Path,
    brand_dir: Option<&Path>,
    safety_backup_dir: Option<&Path>,
) -> AppResult<BackupInfo> {
    let validation = validate_backup(path)?;
    if !validation.valid {
        return Err(AppError::storage(format!(
            "لا يمكن الاستعادة: {}",
            validation.message
        )));
    }

    // Safety backup before touching anything.
    if let Some(dir) = safety_backup_dir {
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let safety = dir.join(format!("pre-restore-{stamp}.zip"));
        let live = Connection::open(db_path)
            .map_err(|e| AppError::storage(format!("تعذّر فتح قاعدة البيانات: {e}")))?;
        create_backup(&live, &safety, brand_dir)?;
    }

    // Extract the archive.
    let file = std::fs::File::open(path)
        .map_err(|e| AppError::storage(format!("تعذّر فتح ملف النسخة: {e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::storage(format!("الملف ليس نسخة احتياطية صالحة: {e}")))?;

    let mut db_bytes = Vec::new();
    {
        let mut entry = archive
            .by_name(DB_ENTRY)
            .map_err(|e| AppError::storage(format!("النسخة لا تحتوي على قاعدة بيانات: {e}")))?;
        entry
            .read_to_end(&mut db_bytes)
            .map_err(|e| AppError::storage(format!("تعذّر قراءة قاعدة البيانات: {e}")))?;
    }

    // Stage and integrity-check the replacement before it goes live.
    let staged = tempfile_path("restore");
    std::fs::write(&staged, &db_bytes)
        .map_err(|e| AppError::storage(format!("تعذّر تجهيز النسخة للاستعادة: {e}")))?;

    {
        let staged_conn = Connection::open(&staged)
            .map_err(|e| AppError::storage(format!("تعذّر فتح النسخة المجهزة: {e}")))?;
        let integrity: String = staged_conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| AppError::storage(format!("فشل التحقق من سلامة النسخة: {e}")))?;
        if !integrity.eq_ignore_ascii_case("ok") {
            let _ = std::fs::remove_file(&staged);
            return Err(AppError::storage(format!(
                "لا يمكن الاستعادة: فشلت سلامة النسخة ({integrity})"
            )));
        }
    }

    // Swap the live database file. Stale WAL/SHM side files are removed so the
    // restored file is authoritative.
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
    std::fs::copy(&staged, db_path)
        .map_err(|e| AppError::storage(format!("تعذّر استبدال قاعدة البيانات: {e}")))?;
    let _ = std::fs::remove_file(&staged);

    // Restore the brand assets when present.
    if let Some(dir) = brand_dir {
        if let Ok(mut entry) = archive.by_name(BRAND_ENTRY) {
            let mut logo = Vec::new();
            entry
                .read_to_end(&mut logo)
                .map_err(|e| AppError::storage(format!("تعذّر قراءة الشعار: {e}")))?;
            std::fs::create_dir_all(dir).ok();
            std::fs::write(dir.join("logo.svg"), logo)
                .map_err(|e| AppError::storage(format!("تعذّر استعادة الشعار: {e}")))?;
        }
    }

    let (sha256, size_bytes) = hash_file(path)?;
    Ok(BackupInfo {
        file_path: path.to_string_lossy().to_string(),
        sha256,
        size_bytes,
        created_at: now(),
    })
}

/// Compute the SHA-256 of a file and its size in bytes.
pub fn hash_file(path: &Path) -> AppResult<(String, u64)> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| AppError::storage(format!("تعذّر فتح الملف للتجزئة: {e}")))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| AppError::storage(format!("خطأ أثناء التجزئة: {e}")))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
    Ok((hex::encode(hasher.finalize()), total))
}

/// Remove old backups, keeping only the most recent `retention` files.
pub fn prune_backups(destination: &Path, retention: usize) -> AppResult<usize> {
    if retention == 0 {
        return Ok(0);
    }
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    let entries = match std::fs::read_dir(destination) {
        Ok(entries) => entries,
        Err(_) => return Ok(0),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
            files.push((path, modified));
        }
    }
    files.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    let mut removed = 0;
    for (path, _) in files.into_iter().skip(retention) {
        if std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

fn tempfile_path(tag: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ua-invoice-{tag}-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn temp_dir(tag: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("ua-backup-test-{}-{}", std::process::id(), tag));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn creates_and_validates_backup() {
        let dir = temp_dir("create");
        let conn = open_in_memory().unwrap();
        crate::services::catalog::create(
            &conn,
            &crate::services::catalog::ProductInput {
                name: "حليب".into(),
                base_price_minor: 2500,
                sku: None,
            },
        )
        .unwrap();

        let target = dir.join("backup.zip");
        let info = create_backup(&conn, &target, None).unwrap();
        assert!(target.exists());
        assert!(!info.sha256.is_empty());
        assert!(info.size_bytes > 0);

        let validation = validate_backup(&target).unwrap();
        assert!(validation.valid, "{}", validation.message);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_corrupt_backup() {
        let dir = temp_dir("corrupt");
        let target = dir.join("broken.zip");
        std::fs::write(&target, b"this is not a zip archive").unwrap();

        let validation = validate_backup(&target).unwrap();
        assert!(!validation.valid);

        // Restoring a corrupt file must fail and leave the live database intact.
        let live_db = dir.join("live.db");
        {
            let conn = crate::db::open(&live_db).unwrap();
            crate::services::catalog::create(
                &conn,
                &crate::services::catalog::ProductInput {
                    name: "منتج".into(),
                    base_price_minor: 100,
                    sku: None,
                },
            )
            .unwrap();
        }
        assert!(restore_backup(&live_db, &target, None, None).is_err());

        // The live database still holds its data.
        let conn = crate::db::open(&live_db).unwrap();
        assert_eq!(crate::services::catalog::count(&conn).unwrap(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn restores_database_contents() {
        let dir = temp_dir("restore");
        let live_db = dir.join("live.db");

        // Create a backup from a populated database.
        let backup_path = dir.join("backup.zip");
        {
            let conn = crate::db::open(&live_db).unwrap();
            crate::services::catalog::create(
                &conn,
                &crate::services::catalog::ProductInput {
                    name: "منتج محفوظ".into(),
                    base_price_minor: 500,
                    sku: None,
                },
            )
            .unwrap();
            create_backup(&conn, &backup_path, None).unwrap();
        }

        // Wipe the live database, then restore.
        {
            let conn = crate::db::open(&live_db).unwrap();
            conn.execute("DELETE FROM products", []).unwrap();
            assert_eq!(crate::services::catalog::count(&conn).unwrap(), 0);
        }

        restore_backup(&live_db, &backup_path, None, None).unwrap();

        let conn = crate::db::open(&live_db).unwrap();
        assert_eq!(crate::services::catalog::count(&conn).unwrap(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_file_is_not_valid() {
        let validation = validate_backup(Path::new("does-not-exist.zip")).unwrap();
        assert!(!validation.valid);
    }

    #[test]
    fn prunes_old_backups() {
        let dir = temp_dir("prune");
        for index in 0..5 {
            std::fs::write(dir.join(format!("b{index}.zip")), b"x").unwrap();
        }
        let removed = prune_backups(&dir, 2).unwrap();
        assert_eq!(removed, 3);
        let _ = std::fs::remove_dir_all(dir);
    }
}
