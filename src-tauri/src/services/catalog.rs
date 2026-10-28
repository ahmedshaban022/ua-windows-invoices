//! Product catalog service (FR-02, FR-04).
//!
//! Search uses an FTS5 `trigram` index over the normalized name, which gives
//! true substring matching at 10,000+ products. Queries shorter than three
//! characters fall back to a bounded `LIKE` scan.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::now;
use crate::domain::arabic::normalize_search;
use crate::domain::error::{AppError, AppResult};

/// A product as shown in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub base_price_minor: i64,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A product plus its catalog price, used by the invoice combobox.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductSuggestion {
    pub id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub base_price_minor: i64,
}

/// A page of products.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPage {
    pub items: Vec<Product>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Input for creating or updating a product.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub name: String,
    pub base_price_minor: i64,
    #[serde(default)]
    pub sku: Option<String>,
}

impl ProductInput {
    fn validate(&self) -> AppResult<()> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(AppError::validation("اسم المنتج مطلوب"));
        }
        if name.chars().count() > 300 {
            return Err(AppError::validation("اسم المنتج طويل جدًا"));
        }
        if self.base_price_minor < 0 {
            return Err(AppError::validation("السعر لا يمكن أن يكون سالبًا"));
        }
        Ok(())
    }
}

/// Maximum results returned to the invoice combobox (FR-02: 8–10 suggestions).
pub const COMBOBOX_LIMIT: i64 = 10;
/// Default catalog page size.
pub const PAGE_SIZE: i64 = 25;

/// Create a product.
pub fn create(conn: &Connection, input: &ProductInput) -> AppResult<Product> {
    input.validate()?;
    let name = input.name.trim().to_string();
    let normalized = normalize_search(&name);
    let sku = input
        .sku
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let timestamp = now();

    conn.execute(
        "INSERT INTO products(name, normalized_name, sku, base_price_minor, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            name,
            normalized,
            sku,
            input.base_price_minor,
            timestamp,
            timestamp
        ],
    )?;

    let id = conn.last_insert_rowid();
    get(conn, id)?.ok_or_else(|| AppError::internal("تعذّر إنشاء المنتج"))
}

/// Fetch a single product.
pub fn get(conn: &Connection, id: i64) -> AppResult<Option<Product>> {
    let product = conn
        .query_row(
            "SELECT id, name, sku, base_price_minor, deleted_at, created_at, updated_at
             FROM products WHERE id = ?1",
            [id],
            map_product,
        )
        .optional()?;
    Ok(product)
}

/// Update a product's name, price and SKU.
pub fn update(conn: &Connection, id: i64, input: &ProductInput) -> AppResult<Product> {
    input.validate()?;
    let name = input.name.trim().to_string();
    let normalized = normalize_search(&name);
    let sku = input
        .sku
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let changed = conn.execute(
        "UPDATE products SET name = ?2, normalized_name = ?3, sku = ?4,
             base_price_minor = ?5, updated_at = ?6
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, name, normalized, sku, input.base_price_minor, now()],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("المنتج غير موجود"));
    }
    get(conn, id)?.ok_or_else(|| AppError::internal("تعذّر تحديث المنتج"))
}

/// Soft-delete a product. Previously issued invoices keep their own snapshots,
/// so deleting a product never alters history (FR-04, FR-06).
pub fn soft_delete(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE products SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
        params![id, now()],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("المنتج غير موجود"));
    }
    Ok(())
}

/// Restore a soft-deleted product.
pub fn restore(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE products SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NOT NULL",
        params![id, now()],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("لا يوجد منتج محذوف بهذا المعرّف"));
    }
    Ok(())
}

/// Search the catalog. Returns a page of products ordered by relevance then name.
pub fn search(
    conn: &Connection,
    query: &str,
    page: i64,
    page_size: i64,
    include_deleted: bool,
) -> AppResult<ProductPage> {
    let page = page.max(1);
    let page_size = page_size.clamp(1, 200);
    let trimmed = query.trim();
    let normalized = normalize_search(trimmed);

    if normalized.is_empty() {
        let total: i64 = if include_deleted {
            conn.query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM products WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )?
        };
        let mut stmt = conn.prepare(
            "SELECT id, name, sku, base_price_minor, deleted_at, created_at, updated_at
             FROM products
             WHERE (?1 = 1 OR deleted_at IS NULL)
             ORDER BY name COLLATE NOCASE ASC
             LIMIT ?2 OFFSET ?3",
        )?;
        let items = stmt
            .query_map(
                params![include_deleted as i64, page_size, (page - 1) * page_size],
                map_product,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(ProductPage {
            items,
            total,
            page,
            page_size,
        });
    }

    // FTS5 MATCH requires at least three characters.
    if normalized.chars().count() >= 3 {
        search_fts(conn, &normalized, page, page_size, include_deleted)
    } else {
        search_like(conn, &normalized, page, page_size, include_deleted)
    }
}

fn search_fts(
    conn: &Connection,
    normalized: &str,
    page: i64,
    page_size: i64,
    include_deleted: bool,
) -> AppResult<ProductPage> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM products_fts f JOIN products p ON p.id = f.rowid
         WHERE products_fts MATCH ?1 AND (?2 = 1 OR p.deleted_at IS NULL)",
        params![normalized, include_deleted as i64],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.sku, p.base_price_minor, p.deleted_at, p.created_at, p.updated_at
         FROM products_fts f JOIN products p ON p.id = f.rowid
         WHERE products_fts MATCH ?1 AND (?4 = 1 OR p.deleted_at IS NULL)
         ORDER BY bm25(products_fts), p.name COLLATE NOCASE
         LIMIT ?2 OFFSET ?3",
    )?;
    let items = stmt
        .query_map(
            params![
                normalized,
                page_size,
                (page - 1) * page_size,
                include_deleted as i64
            ],
            map_product,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProductPage {
        items,
        total,
        page,
        page_size,
    })
}

fn search_like(
    conn: &Connection,
    normalized: &str,
    page: i64,
    page_size: i64,
    include_deleted: bool,
) -> AppResult<ProductPage> {
    let pattern = format!("%{normalized}%");
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM products
         WHERE normalized_name LIKE ?1 ESCAPE '\\' AND (?2 = 1 OR deleted_at IS NULL)",
        params![pattern, include_deleted as i64],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, name, sku, base_price_minor, deleted_at, created_at, updated_at
         FROM products
         WHERE normalized_name LIKE ?1 ESCAPE '\\' AND (?4 = 1 OR deleted_at IS NULL)
         ORDER BY name COLLATE NOCASE
         LIMIT ?2 OFFSET ?3",
    )?;
    let items = stmt
        .query_map(
            params![
                pattern,
                page_size,
                (page - 1) * page_size,
                include_deleted as i64
            ],
            map_product,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProductPage {
        items,
        total,
        page,
        page_size,
    })
}

/// Suggestions for the invoice combobox (FR-02): at most 8–10 rows.
pub fn suggest(conn: &Connection, query: &str, limit: i64) -> AppResult<Vec<ProductSuggestion>> {
    let limit = limit.clamp(1, COMBOBOX_LIMIT);
    let normalized = normalize_search(query);

    if normalized.is_empty() {
        let mut stmt = conn.prepare(
            "SELECT id, name, sku, base_price_minor FROM products
             WHERE deleted_at IS NULL
             ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit], map_suggestion)?
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(rows);
    }

    if normalized.chars().count() >= 3 {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.sku, p.base_price_minor
             FROM products_fts f JOIN products p ON p.id = f.rowid
             WHERE products_fts MATCH ?1 AND p.deleted_at IS NULL
             ORDER BY bm25(products_fts), p.name COLLATE NOCASE
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![normalized, limit], map_suggestion)?
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(rows);
    }

    let pattern = format!("%{normalized}%");
    let mut stmt = conn.prepare(
        "SELECT id, name, sku, base_price_minor FROM products
         WHERE normalized_name LIKE ?1 ESCAPE '\\' AND deleted_at IS NULL
         ORDER BY name COLLATE NOCASE LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![pattern, limit], map_suggestion)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Count of live products.
pub fn count(conn: &Connection) -> AppResult<i64> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM products WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    Ok(count)
}

fn map_product(row: &rusqlite::Row<'_>) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get(0)?,
        name: row.get(1)?,
        sku: row.get(2)?,
        base_price_minor: row.get(3)?,
        deleted_at: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn map_suggestion(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductSuggestion> {
    Ok(ProductSuggestion {
        id: row.get(0)?,
        name: row.get(1)?,
        sku: row.get(2)?,
        base_price_minor: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn input(name: &str, price: i64) -> ProductInput {
        ProductInput {
            name: name.into(),
            base_price_minor: price,
            sku: None,
        }
    }

    #[test]
    fn creates_and_reads_product() {
        let conn = open_in_memory().unwrap();
        let product = create(&conn, &input("حليب كامل الدسم", 2500)).unwrap();
        assert_eq!(product.name, "حليب كامل الدسم");
        assert_eq!(product.base_price_minor, 2500);
        assert!(get(&conn, product.id).unwrap().is_some());
    }

    #[test]
    fn rejects_empty_name_and_negative_price() {
        let conn = open_in_memory().unwrap();
        assert!(create(&conn, &input("  ", 100)).is_err());
        assert!(create(&conn, &input("منتج", -1)).is_err());
    }

    #[test]
    fn searches_partial_arabic_substring() {
        let conn = open_in_memory().unwrap();
        create(&conn, &input("حليب كامل الدسم", 2500)).unwrap();
        create(&conn, &input("خبز بلدي", 1000)).unwrap();

        let page = search(&conn, "حليب", 1, 25, false).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].name, "حليب كامل الدسم");

        // Mid-word substring, which only the trigram index can serve.
        let page = search(&conn, "كامل", 1, 25, false).unwrap();
        assert_eq!(page.total, 1);
    }

    #[test]
    fn search_normalizes_hamza_variants() {
        let conn = open_in_memory().unwrap();
        create(&conn, &input("أحمد", 1000)).unwrap();
        // Querying with the base alef must still match.
        let page = search(&conn, "احمد", 1, 25, false).unwrap();
        assert_eq!(page.total, 1);
    }

    #[test]
    fn short_queries_fall_back_to_like() {
        let conn = open_in_memory().unwrap();
        create(&conn, &input("عصير برتقال", 1500)).unwrap();
        // Two characters are too short for the trigram index, so the search
        // falls back to a LIKE scan and still finds the product.
        let page = search(&conn, "بر", 1, 25, false).unwrap();
        assert_eq!(page.total, 1);
    }

    #[test]
    fn soft_delete_hides_product_but_keeps_row() {
        let conn = open_in_memory().unwrap();
        let product = create(&conn, &input("منتج", 100)).unwrap();
        soft_delete(&conn, product.id).unwrap();

        let page = search(&conn, "", 1, 25, false).unwrap();
        assert_eq!(page.total, 0);

        let page = search(&conn, "", 1, 25, true).unwrap();
        assert_eq!(page.total, 1);

        restore(&conn, product.id).unwrap();
        assert_eq!(count(&conn).unwrap(), 1);
    }

    #[test]
    fn suggestions_are_capped() {
        let conn = open_in_memory().unwrap();
        for index in 0..30 {
            create(&conn, &input(&format!("منتج {index}"), 100)).unwrap();
        }
        let suggestions = suggest(&conn, "منتج", COMBOBOX_LIMIT).unwrap();
        assert_eq!(suggestions.len(), COMBOBOX_LIMIT as usize);
    }

    #[test]
    fn updates_product_and_keeps_fts_in_sync() {
        let conn = open_in_memory().unwrap();
        let product = create(&conn, &input("اسم قديم", 100)).unwrap();
        update(&conn, product.id, &input("اسم جديد", 250)).unwrap();

        assert_eq!(search(&conn, "جديد", 1, 25, false).unwrap().total, 1);
        assert_eq!(search(&conn, "قديم", 1, 25, false).unwrap().total, 0);
    }
}
