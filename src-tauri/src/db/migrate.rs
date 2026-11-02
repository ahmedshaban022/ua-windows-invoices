//! Migration runner.
//!
//! Migrations are embedded at compile time, applied in order inside a
//! transaction, and recorded in `schema_migrations`. Each migration is
//! forward-only and re-runnable (PRD §7).

use rusqlite::Connection;

use crate::domain::error::AppResult;

/// Every migration, in order. The version number is the file prefix.
const MIGRATIONS: &[(i64, &str, &str)] = &[
    (1, "0001_init.sql", include_str!("migrations/0001_init.sql")),
    (
        2,
        "0002_invoice_terms.sql",
        include_str!("migrations/0002_invoice_terms.sql"),
    ),
    (
        3,
        "0003_invoice_discount.sql",
        include_str!("migrations/0003_invoice_discount.sql"),
    ),
];

/// Apply all pending migrations.
pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL
         );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    for (version, name, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![version, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
        log::info!("applied migration {version} ({name})");
    }

    Ok(())
}

/// The highest applied migration version.
pub fn current_version(conn: &Connection) -> AppResult<i64> {
    let version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_to_empty_database() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 3);

        // Core tables exist.
        for table in [
            "products",
            "products_fts",
            "invoices",
            "invoice_items",
            "app_settings",
            "invoice_number_sequence",
            "drafts",
            "backup_history",
            "update_preferences",
        ] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing table {table}");
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        run(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 3);
    }

    #[test]
    fn invoices_carry_a_terms_column() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(invoices)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(columns.iter().any(|c| c == "terms_json"));
    }

    #[test]
    fn invoices_carry_the_discount_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(invoices)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(columns.iter().any(|c| c == "discount_kind"));
        assert!(columns.iter().any(|c| c == "discount_value"));
        assert!(columns.iter().any(|c| c == "discount_minor"));
    }

    #[test]
    fn an_existing_database_gains_the_discount_columns() {
        let conn = Connection::open_in_memory().unwrap();

        // The schema as it was before the discount existed, with an invoice in
        // it — this is what an upgrading install looks like.
        conn.execute_batch(include_str!("migrations/0001_init.sql"))
            .unwrap();
        conn.execute_batch(include_str!("migrations/0002_invoice_terms.sql"))
            .unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 version    INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL
             );
             INSERT INTO schema_migrations(version, applied_at)
                 VALUES (1, '2026-01-01'), (2, '2026-01-01');
             INSERT INTO invoices(invoice_date, customer_name, status, seller_snapshot_json,
                     total_minor, created_at, updated_at)
                 VALUES ('2026-01-01', 'عميل قديم', 'issued', '{}', 5000, '2026-01-01', '2026-01-01');",
        )
        .unwrap();

        run(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 3);

        // The old invoice keeps its total and gains an empty discount.
        let row: (i64, String, i64, i64) = conn
            .query_row(
                "SELECT total_minor, discount_kind, discount_value, discount_minor
                 FROM invoices WHERE customer_name = 'عميل قديم'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(row.0, 5000);
        assert_eq!(row.1, "amount");
        assert_eq!(row.2, 0);
        assert_eq!(row.3, 0);
    }

    #[test]
    fn number_sequence_is_seeded() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let value: i64 = conn
            .query_row(
                "SELECT value FROM invoice_number_sequence WHERE name = 'invoices'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, 0);
    }
}
