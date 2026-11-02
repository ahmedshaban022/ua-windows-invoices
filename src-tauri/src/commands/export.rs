//! Export commands: PDF, print and PNG/JPG images (FR-07).
//!
//! All three outputs are produced from the **same** canonical document that the
//! on-screen preview renders, through a dedicated, chrome-free "render host"
//! webview.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::domain::document::InvoiceDocument;
use crate::domain::error::{AppError, AppResult};
use crate::export::{self, ImageExportResult, ImageFormat};
use crate::services::invoicing::Invoice;
use crate::state::AppState;

/// The label of the hidden webview that renders the invoice document.
pub const RENDER_HOST_LABEL: &str = "render-host";

/// The document the frontend previews.
#[tauri::command]
pub fn invoice_document(app: AppHandle, invoice_id: i64) -> AppResult<InvoiceDocument> {
    let state = app.state::<AppState>();
    let invoice = state
        .with_conn(|conn| crate::services::invoicing::get(conn, invoice_id))?
        .ok_or_else(|| AppError::not_found("الفاتورة غير موجودة"))?;
    let currency = state.currency()?;
    Ok(InvoiceDocument::build(&invoice, &currency))
}

/// Create a real PDF at the chosen path.
#[tauri::command]
pub async fn export_invoice_pdf(
    app: AppHandle,
    invoice_id: i64,
    path: String,
) -> AppResult<String> {
    let document = load_document(&app, invoice_id)?;
    let host = render_host(&app)?;
    render_document(&host, &document).await?;

    let _guard = export::acquire_export_lock().await?;
    export::windows::print_to_pdf(&host, Path::new(&path)).await?;
    Ok(path)
}

/// Open the native Windows printer dialog for the invoice.
#[tauri::command]
pub async fn print_invoice(app: AppHandle, invoice_id: i64) -> AppResult<()> {
    let document = load_document(&app, invoice_id)?;
    let host = render_host(&app)?;
    render_document(&host, &document).await?;

    let _guard = export::acquire_export_lock().await?;
    export::windows::show_print_dialog(&host).await?;
    Ok(())
}

/// Export the invoice as one PNG/JPG file per page.
#[tauri::command]
pub async fn export_invoice_images(
    app: AppHandle,
    invoice_id: i64,
    directory: String,
    format: Option<ImageFormat>,
    scale: Option<f64>,
) -> AppResult<ImageExportResult> {
    let document = load_document(&app, invoice_id)?;
    let format = format.unwrap_or(ImageFormat::Png);
    let scale = scale.unwrap_or(2.0).clamp(1.0, 4.0);

    let host = render_host(&app)?;
    render_document(&host, &document).await?;

    let _guard = export::acquire_export_lock().await?;

    let mut pages = export::windows::capture_pages(&host, scale).await?;
    if pages.is_empty() || export::is_blank_png(&pages[0]) {
        // Chromium occasionally refuses to paint a never-shown window. Show it
        // briefly so the compositor produces a frame, then capture again.
        let _ = host.show();
        tokio::time::sleep(Duration::from_millis(700)).await;
        pages = export::windows::capture_pages(&host, scale).await?;
        let _ = host.hide();
    }

    if pages.is_empty() {
        return Err(AppError::export("تعذّر التقاط صفحة الفاتورة"));
    }
    if export::is_blank_png(&pages[0]) {
        return Err(AppError::export(
            "تعذّر التقاط صورة الفاتورة: الصفحة ظهرت فارغة",
        ));
    }

    let directory = PathBuf::from(directory);
    std::fs::create_dir_all(&directory)
        .map_err(|e| AppError::export(format!("تعذّر إنشاء مجلد الحفظ: {e}")))?;

    let mut files = Vec::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate() {
        let name = export::page_file_name(&document.invoice_number, index + 1, format);
        let target = directory.join(name);
        let bytes = match format {
            ImageFormat::Png => page.clone(),
            ImageFormat::Jpg => export::png_to_opaque_jpg(page)?,
        };
        std::fs::write(&target, &bytes)
            .map_err(|e| AppError::export(format!("تعذّر حفظ الصورة: {e}")))?;
        files.push(target.to_string_lossy().to_string());
    }

    Ok(ImageExportResult {
        files,
        page_count: pages.len(),
    })
}

/// The payload the frontend sends to preview an invoice that is not saved yet.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    pub invoice_number: String,
    pub invoice_date: String,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub notes: Option<String>,
    /// The custom terms (بنود) chosen for this invoice.
    pub terms: Option<Vec<String>>,
    /// The discount applied to the whole invoice.
    pub discount: Option<crate::domain::invoice::DiscountInput>,
    pub lines: Vec<crate::domain::invoice::InvoiceLineInput>,
}

/// The document a *draft* preview renders (no permanent number yet).
#[tauri::command]
pub fn preview_document(app: AppHandle, payload: PreviewRequest) -> AppResult<InvoiceDocument> {
    let state = app.state::<AppState>();
    let currency = state.currency()?;
    let seller = state.seller_snapshot()?;
    let discount = crate::domain::invoice::Discount::parse(payload.discount.as_ref(), &currency)?;
    let computed = crate::domain::invoice::compute_invoice(&payload.lines, &discount, &currency)?;
    let terms = payload.terms.unwrap_or_default();
    Ok(InvoiceDocument::build_preview(
        crate::domain::document::PreviewInput {
            invoice_number: &payload.invoice_number,
            invoice_date: &payload.invoice_date,
            customer_name: &payload.customer_name,
            customer_phone: payload.customer_phone.as_deref(),
            notes: payload.notes.as_deref(),
            terms: &terms,
            computed: &computed,
        },
        &currency,
        &seller,
    ))
}

/// Reveal the exported file in the Windows Explorer folder.
#[tauri::command]
pub fn open_containing_folder(path: String) -> AppResult<()> {
    let target = PathBuf::from(&path);
    let folder = target.parent().unwrap_or(Path::new(".")).to_path_buf();
    std::process::Command::new("explorer.exe")
        .arg("/select,")
        .arg(&target)
        .spawn()
        .map_err(|e| AppError::io(format!("تعذّر فتح المجلد: {e}")))?;
    let _ = folder;
    Ok(())
}

/// Load the stored invoice and build its canonical document.
fn load_document(app: &AppHandle, invoice_id: i64) -> AppResult<InvoiceDocument> {
    let state = app.state::<AppState>();
    let invoice: Invoice = state
        .with_conn(|conn| crate::services::invoicing::get(conn, invoice_id))?
        .ok_or_else(|| AppError::not_found("الفاتورة غير موجودة"))?;
    let currency = state.currency()?;
    Ok(InvoiceDocument::build(&invoice, &currency))
}

/// The hidden render host window.
fn render_host(app: &AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window(RENDER_HOST_LABEL)
        .ok_or_else(|| AppError::export("نافذة تصدير الفاتورة غير جاهزة"))
}

/// Push a document into the render host and wait for it to be laid out.
async fn render_document(host: &WebviewWindow, document: &InvoiceDocument) -> AppResult<()> {
    let json = serde_json::to_string(document)
        .map_err(|e| AppError::export(format!("تعذّر تجهيز صفحة الفاتورة: {e}")))?;

    // Clear the previous document and its ready flag.
    export::windows::evaluate(
        host,
        "window.__renderReset ? window.__renderReset() : null;",
    )
    .await?;

    let script = format!("window.__renderInvoice({json});");
    export::windows::evaluate(host, &script).await?;

    for _ in 0..120 {
        let ready = export::windows::evaluate_json(host, "window.__renderReady === true").await?;
        if ready.as_bool().unwrap_or(false) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Err(AppError::export("انتهت مهلة تجهيز صفحة الفاتورة"))
}

/// A serializable description of a completed export, used by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub path: String,
    pub page_count: usize,
}
