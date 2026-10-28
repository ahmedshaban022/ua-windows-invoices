//! Product catalog commands (FR-02, FR-04).

use tauri::State;

use crate::domain::error::{AppError, AppResult};
use crate::services::catalog::{
    self, Product, ProductInput, ProductPage, ProductSuggestion, COMBOBOX_LIMIT,
};
use crate::state::AppState;

/// Search the catalog. `query` may be empty to list everything.
#[tauri::command]
pub fn list_products(
    state: State<'_, AppState>,
    query: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
    include_deleted: Option<bool>,
) -> AppResult<ProductPage> {
    let query = query.unwrap_or_default();
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(catalog::PAGE_SIZE);
    let include_deleted = include_deleted.unwrap_or(false);
    state.with_conn(|conn| catalog::search(conn, &query, page, page_size, include_deleted))
}

/// Fetch a single product.
#[tauri::command]
pub fn get_product(state: State<'_, AppState>, id: i64) -> AppResult<Option<Product>> {
    state.with_conn(|conn| catalog::get(conn, id))
}

/// Create a product.
#[tauri::command]
pub fn create_product(state: State<'_, AppState>, input: ProductInput) -> AppResult<Product> {
    state.with_conn(|conn| catalog::create(conn, &input))
}

/// Update a product.
#[tauri::command]
pub fn update_product(
    state: State<'_, AppState>,
    id: i64,
    input: ProductInput,
) -> AppResult<Product> {
    state.with_conn(|conn| catalog::update(conn, id, &input))
}

/// Move a product to the trash. Issued invoices keep their own snapshots.
#[tauri::command]
pub fn delete_product(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|conn| catalog::soft_delete(conn, id))
}

/// Restore a product from the trash.
#[tauri::command]
pub fn restore_product(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|conn| catalog::restore(conn, id))
}

/// Suggestions for the invoice product combobox (at most 10 rows).
#[tauri::command]
pub fn suggest_products(
    state: State<'_, AppState>,
    query: Option<String>,
    limit: Option<i64>,
) -> AppResult<Vec<ProductSuggestion>> {
    let query = query.unwrap_or_default();
    let limit = limit.unwrap_or(COMBOBOX_LIMIT);
    state.with_conn(|conn| catalog::suggest(conn, &query, limit))
}

/// Number of live products.
#[tauri::command]
pub fn count_products(state: State<'_, AppState>) -> AppResult<i64> {
    state.with_conn(catalog::count)
}

/// Parse a price typed by the user into minor units.
///
/// The frontend uses this so the same rounding rules apply everywhere; the
/// editor still recomputes the real totals in Rust before saving.
#[tauri::command]
pub fn parse_amount(text: String) -> AppResult<i64> {
    let currency = crate::domain::money::Currency::egp();
    crate::domain::money::parse_price(&text, &currency)
        .ok_or_else(|| AppError::validation("القيمة المدخلة غير صحيحة"))
}
