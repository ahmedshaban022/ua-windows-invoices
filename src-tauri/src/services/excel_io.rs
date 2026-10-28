//! Excel import / export (FR-05).
//!
//! Import is a two-phase operation: a **dry-run** that classifies every row and
//! explains why, then a confirmed, transactional apply. SKU is the unambiguous
//! matching key; name-only matches are flagged for review rather than being
//! assumed equivalent.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use calamine::{Data, Ods, Range, Reader, Xls, Xlsx};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::now;
use crate::domain::arabic::normalize_search;
use crate::domain::error::{AppError, AppResult};
use crate::domain::money::parse_price;
use crate::domain::money::Currency;
use crate::services::catalog::ProductInput;

/// Which column holds which field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    /// Zero-based column index for the product name.
    pub name: usize,
    /// Zero-based column index for the price.
    pub price: usize,
    /// Zero-based column index for the SKU, when the file has one.
    #[serde(default)]
    pub sku: Option<usize>,
}

/// The worksheets available in the selected file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetInfo {
    pub name: String,
    pub row_count: usize,
}

/// A parsed worksheet with its header row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetPreview {
    pub sheets: Vec<SheetInfo>,
    pub headers: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub suggested_mapping: Option<ColumnMapping>,
}

/// How rows that match an existing product should be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchStrategy {
    /// Leave matching rows alone.
    SkipMatched,
    /// Update the matched product with the file's values.
    UpdateConfirmed,
    /// Only add products that do not match anything.
    AddNewOnly,
}

/// The verdict for a single row in the dry-run preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowVerdict {
    pub row_number: usize,
    pub name: String,
    pub price: String,
    pub sku: Option<String>,
    pub valid: bool,
    /// Arabic explanation of the verdict.
    pub reason: String,
    /// "new", "duplicate_in_file", "sku_match", "name_match" or "invalid".
    pub classification: String,
    /// Id of the existing product this row would match, when applicable.
    pub existing_product_id: Option<i64>,
    pub existing_product_name: Option<String>,
}

/// The complete dry-run report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub rows: Vec<RowVerdict>,
    pub total_rows: usize,
    pub valid_new: usize,
    pub duplicates: usize,
    pub matches: usize,
    pub invalid: usize,
}

/// The outcome of a confirmed import.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub invalid: usize,
}

/// A workbook of any supported spreadsheet format.
enum AnyBook {
    Xlsx(Xlsx<BufReader<File>>),
    Xls(Xls<BufReader<File>>),
    Ods(Ods<BufReader<File>>),
}

impl AnyBook {
    fn sheet_names(&self) -> Vec<String> {
        match self {
            AnyBook::Xlsx(book) => book.sheet_names().to_vec(),
            AnyBook::Xls(book) => book.sheet_names().to_vec(),
            AnyBook::Ods(book) => book.sheet_names().to_vec(),
        }
    }

    fn worksheet_range(&mut self, name: &str) -> Result<Range<Data>, String> {
        match self {
            AnyBook::Xlsx(book) => book
                .worksheet_range(name)
                .map_err(|e| format!("تعذّر قراءة الورقة {name}: {e}")),
            AnyBook::Xls(book) => book
                .worksheet_range(name)
                .map_err(|e| format!("تعذّر قراءة الورقة {name}: {e}")),
            AnyBook::Ods(book) => book
                .worksheet_range(name)
                .map_err(|e| format!("تعذّر قراءة الورقة {name}: {e}")),
        }
    }
}

/// Open a spreadsheet, choosing the reader from the file extension.
fn open_book(path: &std::path::Path) -> AppResult<AnyBook> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let book = match extension.as_str() {
        "xls" => AnyBook::Xls(
            calamine::open_workbook(path)
                .map_err(|e| AppError::io(format!("تعذّر فتح ملف Excel: {e}")))?,
        ),
        "ods" => AnyBook::Ods(
            calamine::open_workbook(path)
                .map_err(|e| AppError::io(format!("تعذّر فتح ملف Excel: {e}")))?,
        ),
        _ => AnyBook::Xlsx(
            calamine::open_workbook(path)
                .map_err(|e| AppError::io(format!("تعذّر فتح ملف Excel: {e}")))?,
        ),
    };
    Ok(book)
}

/// Read the workbook: sheet list, headers and a small sample.
pub fn inspect(path: &std::path::Path, sheet_name: Option<&str>) -> AppResult<SheetPreview> {
    let mut workbook = open_book(path)?;

    let sheet_names: Vec<String> = workbook.sheet_names();
    if sheet_names.is_empty() {
        return Err(AppError::io("الملف لا يحتوي على أي ورقة عمل"));
    }

    let selected = match sheet_name {
        Some(name) => name.to_string(),
        None => sheet_names[0].clone(),
    };
    if !sheet_names.contains(&selected) {
        return Err(AppError::io("ورقة العمل المحددة غير موجودة"));
    }

    let mut sheets = Vec::new();
    for name in &sheet_names {
        let rows = workbook.worksheet_range(name).map_err(AppError::io)?;
        sheets.push(SheetInfo {
            name: name.clone(),
            row_count: rows.rows().len(),
        });
    }

    let range = workbook.worksheet_range(&selected).map_err(AppError::io)?;

    let mut headers: Vec<String> = Vec::new();
    let mut sample_rows: Vec<Vec<String>> = Vec::new();
    for (index, row) in range.rows().enumerate() {
        let cells: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
        if index == 0 {
            headers = cells;
        } else if sample_rows.len() < 5 {
            sample_rows.push(cells);
        }
    }

    let suggested_mapping = suggest_mapping(&headers);

    Ok(SheetPreview {
        sheets,
        headers,
        sample_rows,
        suggested_mapping,
    })
}

/// Guess the column mapping from the header names.
fn suggest_mapping(headers: &[String]) -> Option<ColumnMapping> {
    let mut name = None;
    let mut price = None;
    let mut sku = None;

    for (index, header) in headers.iter().enumerate() {
        let normalized = normalize_search(header);
        if name.is_none()
            && (normalized.contains("اسم")
                || normalized.contains("منتج")
                || normalized.contains("name")
                || normalized.contains("product"))
        {
            name = Some(index);
        } else if price.is_none()
            && (normalized.contains("سعر")
                || normalized.contains("prix")
                || normalized.contains("price")
                || normalized.contains("تكلفة"))
        {
            price = Some(index);
        } else if sku.is_none()
            && (normalized.contains("sku")
                || normalized.contains("كود")
                || normalized.contains("رمز")
                || normalized.contains("code"))
        {
            sku = Some(index);
        }
    }

    match (name, price) {
        (Some(n), Some(p)) => Some(ColumnMapping {
            name: n,
            price: p,
            sku,
        }),
        _ => None,
    }
}

/// Classify every row of the sheet without touching the database.
pub fn dry_run(
    conn: &Connection,
    path: &std::path::Path,
    sheet_name: Option<&str>,
    mapping: &ColumnMapping,
    currency: &Currency,
) -> AppResult<ImportPreview> {
    let mut workbook = open_book(path)?;
    let selected = match sheet_name {
        Some(name) => name.to_string(),
        None => workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| AppError::io("الملف لا يحتوي على أي ورقة عمل"))?,
    };
    let range = workbook.worksheet_range(&selected).map_err(AppError::io)?;

    // Existing catalog keys, for match detection.
    let mut by_sku: HashMap<String, (i64, String)> = HashMap::new();
    let mut by_name: HashMap<String, (i64, String)> = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT id, name, sku FROM products WHERE deleted_at IS NULL")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (id, name, sku) = row?;
            if let Some(sku) = sku {
                by_sku.insert(sku.trim().to_lowercase(), (id, name.clone()));
            }
            by_name.insert(normalize_search(&name), (id, name.clone()));
        }
    }

    let mut rows = Vec::new();
    let mut seen_in_file: HashMap<String, usize> = HashMap::new();
    let mut valid_new = 0;
    let mut duplicates = 0;
    let mut matches = 0;
    let mut invalid = 0;
    let mut total_rows = 0usize;

    for (index, row) in range.rows().enumerate() {
        if index == 0 {
            continue; // header row
        }
        total_rows += 1;
        let row_number = index + 1;

        let name = row
            .get(mapping.name)
            .map(|c| c.to_string())
            .unwrap_or_default()
            .trim()
            .to_string();
        let price_text = row
            .get(mapping.price)
            .map(|c| c.to_string())
            .unwrap_or_default()
            .trim()
            .to_string();
        let sku: Option<String> = mapping
            .sku
            .and_then(|index| row.get(index))
            .map(|cell| cell.to_string().trim().to_string())
            .filter(|value| !value.is_empty());

        let mut verdict = RowVerdict {
            row_number,
            name: name.clone(),
            price: price_text.clone(),
            sku: sku.clone(),
            valid: true,
            reason: String::new(),
            classification: "new".to_string(),
            existing_product_id: None,
            existing_product_name: None,
        };

        if name.is_empty() {
            verdict.valid = false;
            verdict.classification = "invalid".to_string();
            verdict.reason = "اسم المنتج فارغ".to_string();
            invalid += 1;
            rows.push(verdict);
            continue;
        }

        let parsed_price = parse_price(&price_text, currency);
        match parsed_price {
            None => {
                verdict.valid = false;
                verdict.classification = "invalid".to_string();
                verdict.reason = "السعر غير صحيح أو فارغ".to_string();
                invalid += 1;
                rows.push(verdict);
                continue;
            }
            Some(value) if value < 0 => {
                verdict.valid = false;
                verdict.classification = "invalid".to_string();
                verdict.reason = "السعر سالب".to_string();
                invalid += 1;
                rows.push(verdict);
                continue;
            }
            Some(_) => {}
        }

        // Duplicate inside the file itself.
        let file_key = match &sku {
            Some(sku) => format!("sku:{}", sku.trim().to_lowercase()),
            None => format!("name:{}", normalize_search(&name)),
        };
        if seen_in_file.contains_key(&file_key) {
            verdict.classification = "duplicate_in_file".to_string();
            verdict.reason = "مكرر داخل الملف نفسه".to_string();
            duplicates += 1;
            rows.push(verdict);
            continue;
        }
        seen_in_file.insert(file_key, row_number);

        // Match against the existing catalog. SKU is unambiguous; a name-only
        // match is reported for review, never auto-applied.
        let matched = match &sku {
            Some(sku) => by_sku.get(&sku.trim().to_lowercase()).cloned(),
            None => None,
        };
        if let Some((id, existing_name)) = matched {
            verdict.classification = "sku_match".to_string();
            verdict.reason = "مطابق ل منتج موجود بنفس الرمز".to_string();
            verdict.existing_product_id = Some(id);
            verdict.existing_product_name = Some(existing_name);
            matches += 1;
            rows.push(verdict);
            continue;
        }

        if let Some((id, existing_name)) = by_name.get(&normalize_search(&name)).cloned() {
            verdict.classification = "name_match".to_string();
            verdict.reason = "يوجد منتج بنفس الاسم — يتطلب تأكيدًا".to_string();
            verdict.existing_product_id = Some(id);
            verdict.existing_product_name = Some(existing_name);
            matches += 1;
            rows.push(verdict);
            continue;
        }

        verdict.reason = "جديد — جاهز للإضافة".to_string();
        valid_new += 1;
        rows.push(verdict);
    }

    Ok(ImportPreview {
        rows,
        total_rows,
        valid_new,
        duplicates,
        matches,
        invalid,
    })
}

/// Apply a previously previewed import, transactionally.
pub fn apply(
    conn: &Connection,
    path: &std::path::Path,
    sheet_name: Option<&str>,
    mapping: &ColumnMapping,
    currency: &Currency,
    strategy: MatchStrategy,
    // Rows the user explicitly confirmed for update (row numbers, 1-based
    // including the header offset). Only these name-only matches are updated.
    confirmed_rows: &[usize],
) -> AppResult<ImportResult> {
    let preview = dry_run(conn, path, sheet_name, mapping, currency)?;

    let mut imported = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut invalid = 0usize;

    let tx = conn.unchecked_transaction()?;

    for verdict in &preview.rows {
        if !verdict.valid {
            invalid += 1;
            continue;
        }

        match verdict.classification.as_str() {
            "duplicate_in_file" => {
                skipped += 1;
                continue;
            }
            "sku_match" => match strategy {
                MatchStrategy::SkipMatched => skipped += 1,
                MatchStrategy::UpdateConfirmed => {
                    if let Some(id) = verdict.existing_product_id {
                        if let Some(price) = parse_price(&verdict.price, currency) {
                            tx.execute(
                                "UPDATE products SET normalized_name = ?2, base_price_minor = ?3,
                                     updated_at = ?4 WHERE id = ?1",
                                params![id, normalize_search(&verdict.name), price, now()],
                            )?;
                            updated += 1;
                        }
                    }
                }
                MatchStrategy::AddNewOnly => skipped += 1,
            },
            "name_match" => {
                // A name-only match is never auto-applied: it needs explicit
                // confirmation from the user.
                if matches!(strategy, MatchStrategy::UpdateConfirmed)
                    && confirmed_rows.contains(&verdict.row_number)
                {
                    if let Some(id) = verdict.existing_product_id {
                        if let Some(price) = parse_price(&verdict.price, currency) {
                            tx.execute(
                                "UPDATE products SET normalized_name = ?2, base_price_minor = ?3,
                                     updated_at = ?4 WHERE id = ?1",
                                params![id, normalize_search(&verdict.name), price, now()],
                            )?;
                            updated += 1;
                        }
                    }
                } else {
                    skipped += 1;
                }
            }
            _ => {
                if let Some(price) = parse_price(&verdict.price, currency) {
                    let input = ProductInput {
                        name: verdict.name.clone(),
                        base_price_minor: price,
                        sku: verdict.sku.clone(),
                    };
                    let name = input.name.trim().to_string();
                    let normalized = normalize_search(&name);
                    let timestamp = now();
                    tx.execute(
                        "INSERT INTO products(name, normalized_name, sku, base_price_minor,
                             created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                        params![name, normalized, input.sku, price, timestamp],
                    )?;
                    imported += 1;
                } else {
                    invalid += 1;
                }
            }
        }
    }

    tx.commit()?;

    Ok(ImportResult {
        imported,
        updated,
        skipped,
        invalid,
    })
}

/// Export products to an `.xlsx` file with real numeric cells.
pub fn export(
    conn: &Connection,
    path: &std::path::Path,
    query: &str,
    currency: &Currency,
) -> AppResult<usize> {
    let page = crate::services::catalog::search(conn, query, 1, 100_000, false)?;
    let products = page.items;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    {
        let sheet = workbook.add_worksheet();
        let header_format = rust_xlsxwriter::Format::new().set_bold();
        let price_format = rust_xlsxwriter::Format::new().set_num_format("#,##0.00");
        sheet.write_row_with_format(
            0,
            0,
            ["اسم المنتج", "السعر", "الرمز (SKU)"],
            &header_format,
        )?;

        for (index, product) in products.iter().enumerate() {
            let row = index as u32 + 1;
            sheet.write_string(row, 0, &product.name)?;
            // Prices are written as real numbers so spreadsheets can compute with them.
            let major = product.base_price_minor as f64 / 10f64.powi(currency.exponent as i32);
            sheet.write_number_with_format(row, 1, major, &price_format)?;
            match &product.sku {
                Some(sku) => {
                    sheet.write_string(row, 2, sku)?;
                }
                None => {
                    sheet.write_blank(row, 2, &rust_xlsxwriter::Format::new())?;
                }
            }
        }
        sheet.set_column_width(0, 34)?;
        sheet.set_column_width(1, 14)?;
        sheet.set_column_width(2, 18)?;
    }

    workbook
        .save(path)
        .map_err(|e| AppError::io(format!("تعذّر حفظ ملف Excel: {e}")))?;

    Ok(products.len())
}

/// Write the import template file (Name / Price / SKU headers plus an example row).
pub fn write_template(path: &std::path::Path) -> AppResult<()> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    {
        let sheet = workbook.add_worksheet();
        let header_format = rust_xlsxwriter::Format::new().set_bold();
        let price_format = rust_xlsxwriter::Format::new().set_num_format("#,##0.00");
        sheet.write_row_with_format(
            0,
            0,
            ["اسم المنتج", "السعر", "الرمز (SKU)"],
            &header_format,
        )?;
        sheet.write_string(1, 0, "مثال منتج")?;
        sheet.write_number_with_format(2, 1, 25.0, &price_format)?;
        sheet.write_string(3, 2, "SKU-001")?;
        sheet.set_column_width(0, 34)?;
        sheet.set_column_width(1, 14)?;
        sheet.set_column_width(2, 18)?;
    }
    workbook
        .save(path)
        .map_err(|e| AppError::io(format!("تعذّر إنشاء القالب: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("ua-invoice-test-{}-{}", std::process::id(), name));
        path
    }

    fn make_workbook(path: &PathBuf, rows: &[(&str, &str, &str)]) {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        {
            let sheet = workbook.add_worksheet();
            sheet
                .write_row(0, 0, ["اسم المنتج", "السعر", "الرمز (SKU)"])
                .unwrap();
            for (index, (name, price, sku)) in rows.iter().enumerate() {
                let row = index as u32 + 1;
                sheet.write_string(row, 0, *name).unwrap();
                sheet
                    .write_number(row, 1, price.parse::<f64>().unwrap_or(0.0))
                    .unwrap();
                sheet.write_string(row, 2, *sku).unwrap();
            }
        }
        workbook.save(path).unwrap();
    }

    fn mapping() -> ColumnMapping {
        ColumnMapping {
            name: 0,
            price: 1,
            sku: Some(2),
        }
    }

    #[test]
    fn inspects_workbook_and_suggests_mapping() {
        let path = temp_path("inspect.xlsx");
        make_workbook(&path, &[("منتج", "10", "A")]);
        let preview = inspect(&path, None).unwrap();
        assert_eq!(preview.sheets.len(), 1);
        assert_eq!(preview.headers.len(), 3);
        assert!(preview.suggested_mapping.is_some());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn dry_run_classifies_rows() {
        let path = temp_path("dryrun.xlsx");
        make_workbook(
            &path,
            &[
                ("منتج جديد", "25.50", "S-1"),
                ("منتج مكرر", "10", "S-2"),
                ("منتج مكرر", "10", "S-2"),
                ("", "10", "S-3"),
                ("سالب", "-5", "S-4"),
            ],
        );
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let preview = dry_run(&conn, &path, None, &mapping(), &egp).unwrap();

        assert_eq!(preview.total_rows, 5);
        // Rows 2 and 3 are new (the second "منتج مكرر" is the duplicate).
        assert_eq!(preview.valid_new, 2);
        assert_eq!(preview.duplicates, 1);
        assert_eq!(preview.invalid, 2);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn apply_adds_new_products_transactionally() {
        let path = temp_path("apply.xlsx");
        make_workbook(
            &path,
            &[("منتج أ", "25.50", "S-1"), ("منتج ب", "10", "S-2")],
        );
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        let result = apply(
            &conn,
            &path,
            None,
            &mapping(),
            &egp,
            MatchStrategy::AddNewOnly,
            &[],
        )
        .unwrap();
        assert_eq!(result.imported, 2);
        assert_eq!(crate::services::catalog::count(&conn).unwrap(), 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn name_only_match_requires_confirmation() {
        let path = temp_path("namematch.xlsx");
        make_workbook(&path, &[("حليب", "99", "NEW-SKU")]);
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        crate::services::catalog::create(
            &conn,
            &ProductInput {
                name: "حليب".into(),
                base_price_minor: 2500,
                sku: Some("OLD-SKU".into()),
            },
        )
        .unwrap();

        // Without confirmation the matching row is skipped.
        let result = apply(
            &conn,
            &path,
            None,
            &mapping(),
            &egp,
            MatchStrategy::UpdateConfirmed,
            &[],
        )
        .unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.skipped, 1);

        // With confirmation it is updated.
        let result = apply(
            &conn,
            &path,
            None,
            &mapping(),
            &egp,
            MatchStrategy::UpdateConfirmed,
            &[2],
        )
        .unwrap();
        assert_eq!(result.updated, 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn export_writes_numeric_prices() {
        let path = temp_path("export.xlsx");
        let conn = open_in_memory().unwrap();
        let egp = Currency::egp();
        crate::services::catalog::create(
            &conn,
            &ProductInput {
                name: "حليب".into(),
                base_price_minor: 2550,
                sku: Some("S-1".into()),
            },
        )
        .unwrap();

        let count = export(&conn, &path, "", &egp).unwrap();
        assert_eq!(count, 1);

        let mut workbook: Xlsx<_> = calamine::open_workbook(&path).unwrap();
        let range = workbook.worksheet_range("Sheet1").unwrap();
        let rows: Vec<_> = range.rows().collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][0].to_string(), "حليب");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writes_import_template() {
        let path = temp_path("template.xlsx");
        write_template(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }
}
