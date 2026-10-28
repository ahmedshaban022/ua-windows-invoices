//! Invoicing service (FR-01, FR-06, FR-08).
//!
//! Issuing an invoice allocates a permanent number from a durable counter
//! **inside the same transaction** as the invoice insert, and captures the
//! seller/brand snapshot so later setting changes cannot rewrite history.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::now;
use crate::domain::error::{AppError, AppResult};
use crate::domain::invoice::{compute_invoice, InvoiceLineInput, SellerSnapshot};
use crate::domain::money::Currency;
use crate::domain::numbering::{draft_identifier, format_invoice_number};

/// Invoice status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InvoiceStatus {
    Draft,
    Issued,
}

impl InvoiceStatus {
    /// The value stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Issued => "issued",
        }
    }

    fn parse(text: &str) -> Self {
        match text {
            "issued" => InvoiceStatus::Issued,
            _ => InvoiceStatus::Draft,
        }
    }
}

/// A stored invoice line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredLine {
    pub id: i64,
    pub product_id: Option<i64>,
    pub name: String,
    pub quantity_thousandths: i64,
    pub quantity_display: String,
    pub unit_price_minor: i64,
    pub line_total_minor: i64,
    pub sort_order: i32,
}

/// A stored invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub id: i64,
    pub invoice_number: Option<String>,
    /// The identifier shown to the user; a provisional id for drafts.
    pub display_number: String,
    pub invoice_date: String,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub notes: Option<String>,
    pub status: InvoiceStatus,
    pub seller_snapshot: SellerSnapshot,
    pub total_minor: i64,
    pub lines: Vec<StoredLine>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Fields the user supplies when creating or issuing an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceInput {
    pub invoice_date: String,
    pub customer_name: String,
    #[serde(default)]
    pub customer_phone: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub lines: Vec<InvoiceLineInput>,
}

impl InvoiceInput {
    fn validate(&self) -> AppResult<()> {
        let name = self.customer_name.trim();
        if name.is_empty() {
            return Err(AppError::validation("اسم العميل مطلوب"));
        }
        if name.chars().count() > 300 {
            return Err(AppError::validation("اسم العميل طويل جدًا"));
        }
        if self.invoice_date.trim().is_empty() {
            return Err(AppError::validation("تاريخ الفاتورة مطلوب"));
        }
        if let Some(phone) = &self.customer_phone {
            let digits = phone.chars().filter(|c| c.is_ascii_digit()).count();
            if digits == 0 {
                return Err(AppError::validation("رقم الهاتف غير صحيح"));
            }
        }
        Ok(())
    }
}

/// A page of invoices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoicePage {
    pub items: Vec<Invoice>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Filter for the invoice history screen (FR-06).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceFilter {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
}

/// Create a draft invoice. Drafts hold no permanent number (FR-08).
pub fn create_draft(
    conn: &Connection,
    input: &InvoiceInput,
    currency: &Currency,
    seller: &SellerSnapshot,
) -> AppResult<Invoice> {
    input.validate()?;
    let computed = compute_invoice(&input.lines, currency)?;
    let timestamp = now();
    let seller_json = serde_json::to_string(seller)?;

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO invoices(invoice_number, invoice_date, customer_name, customer_phone, notes,
             status, seller_snapshot_json, total_minor, created_at, updated_at)
         VALUES (NULL, ?1, ?2, ?3, ?4, 'draft', ?5, ?6, ?7, ?7)",
        params![
            input.invoice_date.trim(),
            input.customer_name.trim(),
            input
                .customer_phone
                .as_ref()
                .map(|p| p.trim())
                .filter(|p| !p.is_empty()),
            input
                .notes
                .as_ref()
                .map(|n| n.trim())
                .filter(|n| !n.is_empty()),
            seller_json,
            computed.total_minor,
            timestamp,
        ],
    )?;
    let invoice_id = tx.last_insert_rowid();

    insert_lines(&tx, invoice_id, &computed)?;
    tx.commit()?;

    get(conn, invoice_id)?.ok_or_else(|| AppError::internal("تعذّر إنشاء الفاتورة"))
}

/// Issue a draft: allocate a permanent number and freeze the snapshot.
pub fn issue(
    conn: &Connection,
    invoice_id: i64,
    _currency: &Currency,
    seller: &SellerSnapshot,
) -> AppResult<Invoice> {
    let seller_json = serde_json::to_string(seller)?;
    let tx = conn.unchecked_transaction()?;

    // Ensure the invoice exists and is still a draft.
    let status: Option<String> = tx
        .query_row(
            "SELECT status FROM invoices WHERE id = ?1 AND deleted_at IS NULL",
            [invoice_id],
            |row| row.get(0),
        )
        .optional()?;
    match status {
        None => {
            return Err(AppError::not_found("الفاتورة غير موجودة"));
        }
        Some(s) if s == "issued" => {
            return Err(AppError::conflict("الفاتورة صادرة بالفعل"));
        }
        _ => {}
    }

    // Allocate the number atomically with the issue.
    let sequence: i64 = tx.query_row(
        "UPDATE invoice_number_sequence SET value = value + 1 WHERE name = 'invoices' RETURNING value",
        [],
        |row| row.get(0),
    )?;
    let invoice_number = format_invoice_number(sequence);

    tx.execute(
        "UPDATE invoices SET invoice_number = ?2, status = 'issued',
             seller_snapshot_json = ?3, updated_at = ?4
         WHERE id = ?1",
        params![invoice_id, invoice_number, seller_json, now()],
    )?;
    tx.commit()?;

    get(conn, invoice_id)?.ok_or_else(|| AppError::internal("تعذّر إصدار الفاتورة"))
}

/// Update a draft in place.
pub fn update_draft(
    conn: &Connection,
    invoice_id: i64,
    input: &InvoiceInput,
    currency: &Currency,
) -> AppResult<Invoice> {
    input.validate()?;
    let computed = compute_invoice(&input.lines, currency)?;

    let tx = conn.unchecked_transaction()?;
    let status: Option<String> = tx
        .query_row(
            "SELECT status FROM invoices WHERE id = ?1 AND deleted_at IS NULL",
            [invoice_id],
            |row| row.get(0),
        )
        .optional()?;
    match status {
        None => return Err(AppError::not_found("الفاتورة غير موجودة")),
        Some(s) if s == "issued" => return Err(AppError::conflict("لا يمكن تعديل فاتورة صادرة")),
        _ => {}
    }

    tx.execute(
        "UPDATE invoices SET invoice_date = ?2, customer_name = ?3, customer_phone = ?4,
             notes = ?5, total_minor = ?6, updated_at = ?7
         WHERE id = ?1",
        params![
            invoice_id,
            input.invoice_date.trim(),
            input.customer_name.trim(),
            input
                .customer_phone
                .as_ref()
                .map(|p| p.trim())
                .filter(|p| !p.is_empty()),
            input
                .notes
                .as_ref()
                .map(|n| n.trim())
                .filter(|n| !n.is_empty()),
            computed.total_minor,
            now(),
        ],
    )?;
    tx.execute(
        "DELETE FROM invoice_items WHERE invoice_id = ?1",
        [invoice_id],
    )?;
    insert_lines(&tx, invoice_id, &computed)?;
    tx.commit()?;

    get(conn, invoice_id)?.ok_or_else(|| AppError::internal("تعذّر تحديث الفاتورة"))
}

fn insert_lines(
    conn: &Connection,
    invoice_id: i64,
    computed: &crate::domain::invoice::ComputedInvoice,
) -> AppResult<()> {
    for line in &computed.lines {
        conn.execute(
            "INSERT INTO invoice_items(invoice_id, product_id, name_snapshot, quantity_thousandths,
                 unit_price_minor, line_total_minor, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                invoice_id,
                line.product_id,
                line.name,
                line.quantity_thousandths,
                line.unit_price_minor,
                line.line_total_minor,
                line.sort_order,
            ],
        )?;
    }
    Ok(())
}

/// Fetch one invoice with its lines.
pub fn get(conn: &Connection, id: i64) -> AppResult<Option<Invoice>> {
    let invoice = conn
        .query_row(
            "SELECT id, invoice_number, invoice_date, customer_name, customer_phone, notes,
                    status, seller_snapshot_json, total_minor, deleted_at, created_at, updated_at
             FROM invoices WHERE id = ?1",
            [id],
            map_invoice,
        )
        .optional()?;
    match invoice {
        Some(mut invoice) => {
            invoice.lines = load_lines(conn, id)?;
            Ok(Some(invoice))
        }
        None => Ok(None),
    }
}

fn load_lines(conn: &Connection, invoice_id: i64) -> AppResult<Vec<StoredLine>> {
    let mut stmt = conn.prepare(
        "SELECT id, product_id, name_snapshot, quantity_thousandths, unit_price_minor,
                line_total_minor, sort_order
         FROM invoice_items WHERE invoice_id = ?1 ORDER BY sort_order ASC",
    )?;
    let lines = stmt
        .query_map([invoice_id], |row| {
            let quantity_thousandths: i64 = row.get(3)?;
            Ok(StoredLine {
                id: row.get(0)?,
                product_id: row.get(1)?,
                name: row.get(2)?,
                quantity_thousandths,
                quantity_display: format_quantity(quantity_thousandths),
                unit_price_minor: row.get(4)?,
                line_total_minor: row.get(5)?,
                sort_order: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(lines)
}

fn format_quantity(thousandths: i64) -> String {
    let major = thousandths / 1000;
    let frac = thousandths % 1000;
    if frac == 0 {
        format!("{major}")
    } else if frac % 100 == 0 {
        format!("{major}.{}", frac / 100)
    } else if frac % 10 == 0 {
        format!("{major}.{:02}", frac / 10)
    } else {
        format!("{major}.{frac:03}")
    }
}

fn map_invoice(row: &rusqlite::Row<'_>) -> rusqlite::Result<Invoice> {
    let id: i64 = row.get(0)?;
    let invoice_number: Option<String> = row.get(1)?;
    let status_text: String = row.get(6)?;
    let seller_json: String = row.get(7)?;
    let seller_snapshot: SellerSnapshot = serde_json::from_str(&seller_json).unwrap_or_default();
    let display_number = invoice_number
        .clone()
        .unwrap_or_else(|| draft_identifier(id));

    Ok(Invoice {
        id,
        invoice_number,
        display_number,
        invoice_date: row.get(2)?,
        customer_name: row.get(3)?,
        customer_phone: row.get(4)?,
        notes: row.get(5)?,
        status: InvoiceStatus::parse(&status_text),
        seller_snapshot,
        total_minor: row.get(8)?,
        lines: Vec::new(),
        deleted_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

/// Search the invoice history.
pub fn list(conn: &Connection, filter: &InvoiceFilter) -> AppResult<InvoicePage> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(25).clamp(1, 200);
    let query = filter.query.trim().to_string();
    let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));

    let (where_sql, mut params): (String, Vec<rusqlite::types::Value>) = if query.is_empty() {
        (
            "WHERE deleted_at IS NULL AND status = 'issued'".to_string(),
            vec![],
        )
    } else {
        (
            "WHERE deleted_at IS NULL AND status = 'issued'
                   AND (customer_name LIKE ?1 ESCAPE '\\' OR invoice_number LIKE ?1 ESCAPE '\\')"
                .to_string(),
            vec![rusqlite::types::Value::Text(like)],
        )
    };

    let date_clause = build_date_clause(filter, &mut params);

    let total_sql = format!("SELECT COUNT(*) FROM invoices {where_sql}{date_clause}");
    let mut count_stmt = conn.prepare(&total_sql)?;
    let total: i64 =
        count_stmt.query_row(rusqlite::params_from_iter(params.iter()), |r| r.get(0))?;

    let list_sql = format!(
        "SELECT id, invoice_number, invoice_date, customer_name, customer_phone, notes,
                status, seller_snapshot_json, total_minor, deleted_at, created_at, updated_at
         FROM invoices {where_sql}{date_clause}
         ORDER BY invoice_date DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut list_params = params.clone();
    list_params.push(rusqlite::types::Value::Integer(page_size));
    list_params.push(rusqlite::types::Value::Integer((page - 1) * page_size));

    let mut stmt = conn.prepare(&list_sql)?;
    let mut items = stmt
        .query_map(rusqlite::params_from_iter(list_params.iter()), map_invoice)?
        .collect::<Result<Vec<_>, _>>()?;

    for invoice in items.iter_mut() {
        invoice.lines = load_lines(conn, invoice.id)?;
    }

    Ok(InvoicePage {
        items,
        total,
        page,
        page_size,
    })
}

fn build_date_clause(filter: &InvoiceFilter, params: &mut Vec<rusqlite::types::Value>) -> String {
    let mut clause = String::new();
    if let Some(from) = filter.date_from.as_ref().filter(|d| !d.trim().is_empty()) {
        params.push(rusqlite::types::Value::Text(from.trim().to_string()));
        clause.push_str(&format!(" AND invoice_date >= ?{}", params.len()));
    }
    if let Some(to) = filter.date_to.as_ref().filter(|d| !d.trim().is_empty()) {
        params.push(rusqlite::types::Value::Text(to.trim().to_string()));
        clause.push_str(&format!(" AND invoice_date <= ?{}", params.len()));
    }
    clause
}

/// Soft-delete an issued invoice (FR-06). The number is never reused.
pub fn soft_delete(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE invoices SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
        params![id, now()],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("الفاتورة غير موجودة"));
    }
    Ok(())
}

/// Restore a soft-deleted invoice.
pub fn restore(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE invoices SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NOT NULL",
        params![id, now()],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("لا توجد فاتورة محذوفة بهذا المعرّف"));
    }
    Ok(())
}

/// Permanently delete an invoice and its lines.
pub fn purge(conn: &Connection, id: i64) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM invoice_items WHERE invoice_id = ?1", [id])?;
    let changed = tx.execute("DELETE FROM invoices WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::not_found("الفاتورة غير موجودة"));
    }
    tx.commit()?;
    Ok(())
}

/// List soft-deleted invoices (the Trash screen).
pub fn list_trashed(conn: &Connection) -> AppResult<Vec<Invoice>> {
    let mut stmt = conn.prepare(
        "SELECT id, invoice_number, invoice_date, customer_name, customer_phone, notes,
                status, seller_snapshot_json, total_minor, deleted_at, created_at, updated_at
         FROM invoices WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC",
    )?;
    let mut items = stmt
        .query_map([], map_invoice)?
        .collect::<Result<Vec<_>, _>>()?;
    for invoice in items.iter_mut() {
        invoice.lines = load_lines(conn, invoice.id)?;
    }
    Ok(items)
}

/// Duplicate an invoice as a new draft (FR-06). The source is untouched and
/// the copy gets a fresh date and, on issue, a fresh number.
pub fn duplicate_as_new(
    conn: &Connection,
    id: i64,
    currency: &Currency,
    seller: &SellerSnapshot,
) -> AppResult<Invoice> {
    let source = get(conn, id)?.ok_or_else(|| AppError::not_found("الفاتورة غير موجودة"))?;
    let timestamp = now();
    let seller_json = serde_json::to_string(seller)?;

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO invoices(invoice_number, invoice_date, customer_name, customer_phone, notes,
             status, seller_snapshot_json, total_minor, created_at, updated_at)
         VALUES (NULL, ?1, ?2, ?3, ?4, 'draft', ?5, ?6, ?7, ?7)",
        params![
            chrono::Local::now().format("%Y-%m-%d").to_string(),
            source.customer_name,
            source.customer_phone,
            source.notes,
            seller_json,
            source.total_minor,
            timestamp,
        ],
    )?;
    let new_id = tx.last_insert_rowid();

    for line in &source.lines {
        tx.execute(
            "INSERT INTO invoice_items(invoice_id, product_id, name_snapshot, quantity_thousandths,
                 unit_price_minor, line_total_minor, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id,
                line.product_id,
                line.name,
                line.quantity_thousandths,
                line.unit_price_minor,
                line.line_total_minor,
                line.sort_order,
            ],
        )?;
    }
    tx.commit()?;
    let _ = currency;

    get(conn, new_id)?.ok_or_else(|| AppError::internal("تعذّر نسخ الفاتورة"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::invoice::InvoiceLineInput;
    use crate::services::settings::AppSettings;

    fn line(name: &str, qty: &str, price: &str) -> InvoiceLineInput {
        InvoiceLineInput {
            product_id: None,
            name: name.into(),
            quantity: qty.into(),
            unit_price: price.into(),
        }
    }

    fn input() -> InvoiceInput {
        InvoiceInput {
            invoice_date: "2026-10-29".into(),
            customer_name: "عميل تجريبي".into(),
            customer_phone: Some("01001234567".into()),
            notes: None,
            lines: vec![line("حليب", "2", "25.00")],
        }
    }

    fn seller() -> SellerSnapshot {
        let settings = AppSettings::default();
        SellerSnapshot {
            company_name: settings.company_name,
            company_phone: settings.company_phone,
            company_address: settings.company_address,
            logo_svg: settings.logo_svg,
            brand_color: settings.brand_color,
            currency_code: settings.currency_code,
            currency_exponent: settings.currency_exponent,
            date_format: settings.date_format,
        }
    }

    #[test]
    fn draft_has_no_permanent_number() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let draft = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        assert!(draft.invoice_number.is_none());
        assert_eq!(draft.status, InvoiceStatus::Draft);
        assert!(draft.display_number.starts_with("مسودة-"));
    }

    #[test]
    fn issuing_allocates_sequential_numbers() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let draft = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        let issued = issue(&conn, draft.id, &egp, &seller()).unwrap();
        assert_eq!(issued.invoice_number.as_deref(), Some("INV-000001"));
        assert_eq!(issued.status, InvoiceStatus::Issued);

        let second = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        let issued2 = issue(&conn, second.id, &egp, &seller()).unwrap();
        assert_eq!(issued2.invoice_number.as_deref(), Some("INV-000002"));
    }

    #[test]
    fn issuing_twice_is_rejected() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let draft = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        issue(&conn, draft.id, &egp, &seller()).unwrap();
        assert!(issue(&conn, draft.id, &egp, &seller()).is_err());
    }

    #[test]
    fn deleted_numbers_are_not_reused() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let draft = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        let issued = issue(&conn, draft.id, &egp, &seller()).unwrap();
        soft_delete(&conn, issued.id).unwrap();

        let next = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        let issued_next = issue(&conn, next.id, &egp, &seller()).unwrap();
        assert_eq!(issued_next.invoice_number.as_deref(), Some("INV-000002"));
    }

    #[test]
    fn snapshot_survives_product_and_settings_changes() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let product = crate::services::catalog::create(
            &conn,
            &crate::services::catalog::ProductInput {
                name: "حليب".into(),
                base_price_minor: 2500,
                sku: None,
            },
        )
        .unwrap();

        let mut input = input();
        input.lines[0].product_id = Some(product.id);
        let draft = create_draft(&conn, &input, &egp, &seller()).unwrap();
        let issued = issue(&conn, draft.id, &egp, &seller()).unwrap();

        // Change the product and the seller identity afterwards.
        crate::services::catalog::soft_delete(&conn, product.id).unwrap();
        let mut changed_seller = seller();
        changed_seller.company_name = "شركة أخرى".into();

        let reopened = get(&conn, issued.id).unwrap().unwrap();
        assert_eq!(reopened.lines[0].name, "حليب");
        assert_eq!(reopened.lines[0].unit_price_minor, 2500);
        assert_eq!(reopened.seller_snapshot.company_name, "اسم الشركة");
    }

    #[test]
    fn duplicate_creates_new_draft_and_keeps_source() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let draft = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        let issued = issue(&conn, draft.id, &egp, &seller()).unwrap();

        let copy = duplicate_as_new(&conn, issued.id, &egp, &seller()).unwrap();
        assert_eq!(copy.status, InvoiceStatus::Draft);
        assert!(copy.invoice_number.is_none());
        assert_eq!(copy.lines.len(), 1);
        assert_eq!(copy.lines[0].name, issued.lines[0].name);

        // The source is untouched.
        let source = get(&conn, issued.id).unwrap().unwrap();
        assert_eq!(source.status, InvoiceStatus::Issued);
        assert_eq!(source.invoice_number.as_deref(), Some("INV-000001"));
    }

    #[test]
    fn search_filters_by_customer_and_number() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let mut first = input();
        first.customer_name = "محمد أحمد".into();
        let draft = create_draft(&conn, &first, &egp, &seller()).unwrap();
        issue(&conn, draft.id, &egp, &seller()).unwrap();

        let page = list(
            &conn,
            &InvoiceFilter {
                query: "محمد".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 1);

        let page = list(
            &conn,
            &InvoiceFilter {
                query: "INV-000001".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 1);

        let page = list(&conn, &InvoiceFilter::default()).unwrap();
        assert_eq!(page.total, 1);
    }

    #[test]
    fn trash_list_and_purge() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let draft = create_draft(&conn, &input(), &egp, &seller()).unwrap();
        let issued = issue(&conn, draft.id, &egp, &seller()).unwrap();
        soft_delete(&conn, issued.id).unwrap();

        assert_eq!(list_trashed(&conn).unwrap().len(), 1);
        restore(&conn, issued.id).unwrap();
        assert_eq!(list_trashed(&conn).unwrap().len(), 0);

        soft_delete(&conn, issued.id).unwrap();
        purge(&conn, issued.id).unwrap();
        assert!(get(&conn, issued.id).unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_input() {
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let mut bad = input();
        bad.customer_name = "  ".into();
        assert!(create_draft(&conn, &bad, &egp, &seller()).is_err());

        let mut bad = input();
        bad.lines.clear();
        assert!(create_draft(&conn, &bad, &egp, &seller()).is_err());
    }
}
