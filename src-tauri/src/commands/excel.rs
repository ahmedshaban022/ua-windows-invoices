//! Excel import / export commands (FR-05).

use tauri::State;

use crate::domain::error::AppResult;
use crate::services::excel_io::{
    self, ColumnMapping, ImportPreview, ImportResult, MatchStrategy, SheetPreview,
};
use crate::state::AppState;

/// Read a workbook: sheets, headers and a sample of the rows.
#[tauri::command]
pub fn excel_inspect(path: String, sheet_name: Option<String>) -> AppResult<SheetPreview> {
    excel_io::inspect(&std::path::PathBuf::from(path), sheet_name.as_deref())
}

/// Classify every row of the sheet without changing the database.
#[tauri::command]
pub fn excel_dry_run(
    state: State<'_, AppState>,
    path: String,
    sheet_name: Option<String>,
    mapping: ColumnMapping,
) -> AppResult<ImportPreview> {
    let currency = state.currency()?;
    let path = std::path::PathBuf::from(path);
    state.with_conn(|conn| {
        excel_io::dry_run(conn, &path, sheet_name.as_deref(), &mapping, &currency)
    })
}

/// Apply a previewed import transactionally.
#[tauri::command]
pub fn excel_apply(
    state: State<'_, AppState>,
    path: String,
    sheet_name: Option<String>,
    mapping: ColumnMapping,
    strategy: Option<MatchStrategy>,
    confirmed_rows: Option<Vec<usize>>,
) -> AppResult<ImportResult> {
    let currency = state.currency()?;
    let strategy = strategy.unwrap_or(MatchStrategy::SkipMatched);
    let confirmed = confirmed_rows.unwrap_or_default();
    let path = std::path::PathBuf::from(path);
    state.with_conn(|conn| {
        excel_io::apply(
            conn,
            &path,
            sheet_name.as_deref(),
            &mapping,
            &currency,
            strategy,
            &confirmed,
        )
    })
}

/// Export the catalog to an `.xlsx` file with real numeric cells.
#[tauri::command]
pub fn excel_export(
    state: State<'_, AppState>,
    path: String,
    query: Option<String>,
) -> AppResult<usize> {
    let currency = state.currency()?;
    let query = query.unwrap_or_default();
    let path = std::path::PathBuf::from(path);
    state.with_conn(|conn| excel_io::export(conn, &path, &query, &currency))
}

/// Write the import template file.
#[tauri::command]
pub fn excel_write_template(path: String) -> AppResult<()> {
    excel_io::write_template(&std::path::PathBuf::from(path))
}
