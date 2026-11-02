//! Invoice commands (FR-01, FR-06, FR-08).

use tauri::State;

use crate::domain::error::{AppError, AppResult};
use crate::domain::invoice::{
    compute_invoice, ComputedInvoice, Discount, DiscountInput, InvoiceLineInput};
use crate::services::invoicing::{self, Invoice, InvoiceFilter, InvoiceInput, InvoicePage};
use crate::state::AppState;

/// Search the issued-invoice history.
#[tauri::command]
pub fn list_invoices(
    state: State<'_, AppState>,
    filter: Option<InvoiceFilter>,
) -> AppResult<InvoicePage> {
    let filter = filter.unwrap_or_default();
    state.with_conn(|conn| invoicing::list(conn, &filter))
}

/// Fetch one invoice with its lines.
#[tauri::command]
pub fn get_invoice(state: State<'_, AppState>, id: i64) -> AppResult<Option<Invoice>> {
    state.with_conn(|conn| invoicing::get(conn, id))
}

/// Create a draft invoice.
#[tauri::command]
pub fn create_draft(state: State<'_, AppState>, input: InvoiceInput) -> AppResult<Invoice> {
    let currency = state.currency()?;
    let seller = state.seller_snapshot()?;
    state.with_conn(|conn| invoicing::create_draft(conn, &input, &currency, &seller))
}

/// Update a draft invoice in place.
#[tauri::command]
pub fn update_draft(
    state: State<'_, AppState>,
    id: i64,
    input: InvoiceInput,
) -> AppResult<Invoice> {
    let currency = state.currency()?;
    state.with_conn(|conn| invoicing::update_draft(conn, id, &input, &currency))
}

/// Issue a draft: allocate the permanent invoice number and freeze the snapshot.
#[tauri::command]
pub fn issue_invoice(state: State<'_, AppState>, id: i64) -> AppResult<Invoice> {
    let currency = state.currency()?;
    let seller = state.seller_snapshot()?;
    state.with_conn(|conn| invoicing::issue(conn, id, &currency, &seller))
}

/// Recompute totals for the lines currently in the editor.
///
/// The frontend never sends totals; Rust is the single source of truth.
#[tauri::command]
pub fn preview_invoice(
    state: State<'_, AppState>,
    lines: Vec<InvoiceLineInput>,
    discount: Option<DiscountInput>,
) -> AppResult<ComputedInvoice> {
    let currency = state.currency()?;
    let discount = Discount::parse(discount.as_ref(), &currency)?;
    compute_invoice(&lines, &discount, &currency)
}

/// Move an invoice to the trash.
#[tauri::command]
pub fn delete_invoice(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|conn| invoicing::soft_delete(conn, id))
}

/// Restore an invoice from the trash.
#[tauri::command]
pub fn restore_invoice(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|conn| invoicing::restore(conn, id))
}

/// Permanently delete an invoice.
#[tauri::command]
pub fn purge_invoice(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|conn| invoicing::purge(conn, id))
}

/// List the trashed invoices.
#[tauri::command]
pub fn list_trashed_invoices(state: State<'_, AppState>) -> AppResult<Vec<Invoice>> {
    state.with_conn(invoicing::list_trashed)
}

/// Duplicate an invoice as a new draft.
#[tauri::command]
pub fn duplicate_invoice(state: State<'_, AppState>, id: i64) -> AppResult<Invoice> {
    let currency = state.currency()?;
    let seller = state.seller_snapshot()?;
    state.with_conn(|conn| invoicing::duplicate_as_new(conn, id, &currency, &seller))
}

/// Raise a validation error with an Arabic message (used by the frontend's
/// shared error helper so wording stays in one place).
#[tauri::command]
pub fn validation_error(message: String) -> AppResult<()> {
    Err(AppError::validation(message))
}
