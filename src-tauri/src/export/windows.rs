//! Windows WebView2 COM implementation of the export pipeline.
//!
//! Reaches `ICoreWebView2_16` through Tauri 2.12's
//! `PlatformWebview::controller()`, then uses:
//!   * `PrintToPdfStream`        -> a real PDF (not Windows Print-to-PDF)
//!   * `ShowPrintUI(SYSTEM)`     -> the native Windows printer dialog
//!   * CDP `Page.captureScreenshot` -> high-resolution PNG/JPG pages
//!
//! All of these are Chromium's own engines, so Arabic shaping, RTL, inline SVG
//! and A4 pagination match the preview exactly.
//!
//! Every function here is `async`: WebView2 only allows one print job per
//! webview and the completion handler fires on the UI thread, so the calling
//! code must not block the UI thread while it waits.

#![cfg(windows)]

use std::sync::mpsc;
use std::time::Duration;

use tauri::WebviewWindow;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2Environment, ICoreWebView2Environment6,
    ICoreWebView2PrintSettings, ICoreWebView2_16, COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM,
    COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT,
};
use webview2_com::{
    CallDevToolsProtocolMethodCompletedHandler, ExecuteScriptCompletedHandler,
    PrintToPdfStreamCompletedHandler,
};
use windows::core::{Interface, PCWSTR};
use windows::Win32::System::Com::{IStream, STATFLAG_DEFAULT, STATSTG, STREAM_SEEK_SET};

use crate::domain::error::{AppError, AppResult};
use crate::export::{A4_HEIGHT_INCHES, A4_WIDTH_INCHES, RENDER_HOST_HEIGHT, RENDER_HOST_WIDTH};

/// Timeout for a single print / capture operation.
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

/// Print the current document of `window` to a PDF file at `path`.
///
/// Uses `ICoreWebView2_16::PrintToPdfStream`, which produces a genuine PDF
/// with vector text, selectable Arabic, embedded SVG and page breaks derived
/// from the document's own A4 `.page` elements.
pub async fn print_to_pdf(window: &WebviewWindow, path: &std::path::Path) -> AppResult<()> {
    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();
    let tx_for_closure = tx.clone();

    window
        .with_webview(move |platform_webview| {
            let tx = tx_for_closure.clone();
            // The closure runs on the UI thread.
            unsafe {
                if let Err(message) = print_to_pdf_inner(
                    platform_webview.controller(),
                    platform_webview.environment(),
                    tx.clone(),
                ) {
                    let _ = tx.send(Err(message));
                }
            }
        })
        .map_err(|e| AppError::export(format!("تعذّر الوصول إلى نافذة التصدير: {e}")))?;

    let bytes = receive(rx).await?;

    if bytes.is_empty() {
        return Err(AppError::export("لم يتم إنشاء محتوى ملف PDF"));
    }
    if !bytes.starts_with(b"%PDF") {
        return Err(AppError::export("الملف الناتج ليس PDF صالحًا"));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::export(format!("تعذّر إنشاء مجلد الحفظ: {e}")))?;
    }
    std::fs::write(path, &bytes).map_err(|e| AppError::export(format!("تعذّر حفظ ملف PDF: {e}")))?;
    Ok(())
}

/// Open the native Windows printer dialog for the current document.
///
/// Uses `COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM` so the user gets the real
/// Windows printer dialog (PRD FR-07), not Chromium's own preview.
pub async fn show_print_dialog(window: &WebviewWindow) -> AppResult<()> {
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let tx_for_closure = tx.clone();

    window
        .with_webview(move |platform_webview| {
            let tx = tx_for_closure.clone();
            unsafe {
                let outcome = (|| -> Result<(), String> {
                    let core = platform_webview
                        .controller()
                        .CoreWebView2()
                        .map_err(|e| format!("تعذّر الوصول إلى محرك العرض: {e}"))?;
                    let webview_16: ICoreWebView2_16 = core
                        .cast()
                        .map_err(|e| format!("إصدار WebView2 لا يدعم الطباعة: {e}"))?;
                    webview_16
                        .ShowPrintUI(COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM)
                        .map_err(|e| format!("تعذّر فتح نافذة الطباعة: {e}"))?;
                    Ok(())
                })();
                let _ = tx.send(outcome);
            }
        })
        .map_err(|e| AppError::export(format!("تعذّر الوصول إلى نافذة الطباعة: {e}")))?;

    receive(rx).await?;
    Ok(())
}

/// Evaluate JavaScript in the webview and return the raw JSON result text.
pub async fn evaluate(window: &WebviewWindow, script: &str) -> AppResult<String> {
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    let tx_for_closure = tx.clone();
    let script = script.to_string();

    window
        .with_webview(move |platform_webview| {
            let tx = tx_for_closure.clone();
            unsafe {
                let started = (|| -> Result<(), String> {
                    let core = platform_webview
                        .controller()
                        .CoreWebView2()
                        .map_err(|e| format!("تعذّر الوصول إلى محرك العرض: {e}"))?;
                    let handler_tx = tx.clone();
                    let handler = ExecuteScriptCompletedHandler::create(Box::new(
                        move |errorcode, result| {
                            if errorcode.is_err() {
                                let _ = handler_tx
                                    .send(Err("فشل تنفيذ الأمر داخل صفحة الفاتورة".to_string()));
                            } else {
                                let _ = handler_tx.send(Ok(result));
                            }
                            Ok(())
                        },
                    ));
                    let script_hstring = windows::core::HSTRING::from(script);
                    core.ExecuteScript(PCWSTR(script_hstring.as_ptr()), &handler)
                        .map_err(|e| format!("تعذّر تنفيذ الأمر داخل صفحة الفاتورة: {e}"))?;
                    Ok(())
                })();
                if let Err(message) = started {
                    let _ = tx.send(Err(message));
                }
            }
        })
        .map_err(|e| AppError::export(format!("تعذّر تنفيذ الأمر داخل صفحة الفاتورة: {e}")))?;

    receive(rx).await
}

/// Evaluate JavaScript and parse the JSON result into a `serde_json::Value`.
pub async fn evaluate_json(window: &WebviewWindow, script: &str) -> AppResult<serde_json::Value> {
    let raw = evaluate(window, script).await?;
    parse_json_result(&raw)
}

/// Parse the JSON-encoded result of `ExecuteScript`.
///
/// `ExecuteScript` hands back the JSON encoding of the script's result. A
/// script that returns a string is therefore encoded twice: the outer layer
/// is a JSON string that holds JSON. Both shapes are accepted here, so a
/// caller may use `JSON.stringify` without knowing how the result is wrapped.
fn parse_json_result(raw: &str) -> AppResult<serde_json::Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "undefined" || trimmed == "null" {
        return Ok(serde_json::Value::Null);
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| AppError::export(format!("تعذّر قراءة نتيجة صفحة الفاتورة: {e}")))?;

    // A JSON string that carries JSON is decoded one more level.
    if let Some(inner) = value.as_str() {
        let inner = inner.trim();
        if inner.starts_with('[') || inner.starts_with('{') {
            if let Ok(decoded) = serde_json::from_str::<serde_json::Value>(inner) {
                return Ok(decoded);
            }
        }
    }

    Ok(value)
}

/// The bounding box of every page element in the render host, in CSS pixels.
#[derive(Debug, Clone, Copy)]
pub struct PageBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Capture each page of the render host as PNG bytes.
///
/// The pages are captured through the Chrome DevTools Protocol at the given
/// scale factor, one file per page, in page order.
pub async fn capture_pages(window: &WebviewWindow, scale: f64) -> AppResult<Vec<Vec<u8>>> {
    // Pin the layout to A4 at 96 dpi so the page boxes are deterministic.
    let metrics = serde_json::json!({
        "width": RENDER_HOST_WIDTH,
        "height": RENDER_HOST_HEIGHT,
        "deviceScaleFactor": 1,
        "mobile": false,
    });
    call_devtools(window, "Emulation.setDeviceMetricsOverride", &metrics).await?;

    let captured = capture_pages_inner(window, scale).await;

    // Restore the normal metrics so the host window is usable afterwards,
    // even when the capture failed half way through.
    let _ = call_devtools(
        window,
        "Emulation.clearDeviceMetricsOverride",
        &serde_json::json!({}),
    )
    .await;

    captured
}

async fn capture_pages_inner(window: &WebviewWindow, scale: f64) -> AppResult<Vec<Vec<u8>>> {
    let rects = page_boxes(window).await?;

    let mut pages = Vec::with_capacity(rects.len());
    for rect in &rects {
        let params = serde_json::json!({
            "format": "png",
            "captureBeyondViewport": true,
            "fromSurface": true,
            "clip": {
                "x": rect.x,
                "y": rect.y,
                "width": rect.width,
                "height": rect.height,
                "scale": scale,
            }
        });
        let payload = call_devtools(window, "Page.captureScreenshot", &params).await?;
        let data = extract_cdp_data(&payload)?;
        pages.push(decode_base64(&data)?);
    }

    Ok(pages)
}

/// Read the page rectangles reported by the render host.
async fn page_boxes(window: &WebviewWindow) -> AppResult<Vec<PageBox>> {
    // The script returns the array itself: `ExecuteScript` encodes the result
    // as JSON already, so wrapping it in `JSON.stringify` would encode it a
    // second time and the array would arrive as a string.
    const SCRIPT: &str = "Array.from(document.querySelectorAll('[data-invoice-page]')).map(function (el) { var r = el.getBoundingClientRect(); return { x: r.x + window.scrollX, y: r.y + window.scrollY, width: r.width, height: r.height }; })";
    let value = evaluate_json(window, SCRIPT).await?;
    let array = value
        .as_array()
        .ok_or_else(|| AppError::export("تعذّر قراءة صفحات الفاتورة"))?;
    let mut out = Vec::with_capacity(array.len());
    for item in array {
        let x = item.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = item.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let width = item.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let height = item.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if width <= 0.0 || height <= 0.0 {
            continue;
        }
        out.push(PageBox {
            x,
            y,
            width,
            height,
        });
    }
    Ok(out)
}

/// Invoke a Chrome DevTools Protocol method and return its raw JSON result.
async fn call_devtools(
    window: &WebviewWindow,
    method: &str,
    params: &serde_json::Value,
) -> AppResult<String> {
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    let tx_for_closure = tx.clone();
    let method_name = method.to_string();
    let params_json = params.to_string();

    window
        .with_webview(move |platform_webview| {
            let tx = tx_for_closure.clone();
            let method_for_closure = method_name.clone();
            unsafe {
                let started = (|| -> Result<(), String> {
                    let core = platform_webview
                        .controller()
                        .CoreWebView2()
                        .map_err(|e| format!("تعذّر الوصول إلى محرك العرض: {e}"))?;
                    let handler_tx = tx.clone();
                    let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                        move |errorcode, result| {
                            if errorcode.is_err() {
                                let _ =
                                    handler_tx.send(Err(format!("فشل تنفيذ {method_for_closure}")));
                            } else {
                                let _ = handler_tx.send(Ok(result));
                            }
                            Ok(())
                        },
                    ));
                    let method_hstring = windows::core::HSTRING::from(method_name.clone());
                    let params_hstring = windows::core::HSTRING::from(params_json.clone());
                    core.CallDevToolsProtocolMethod(
                        PCWSTR(method_hstring.as_ptr()),
                        PCWSTR(params_hstring.as_ptr()),
                        &handler,
                    )
                    .map_err(|e| format!("تعذّر تنفيذ {method_name}: {e}"))?;
                    Ok(())
                })();
                if let Err(message) = started {
                    let _ = tx.send(Err(message));
                }
            }
        })
        .map_err(|e| AppError::export(format!("تعذّر تنفيذ {method}: {e}")))?;

    receive(rx).await
}

/// Pull the base64 `data` field out of a CDP JSON response.
fn extract_cdp_data(payload: &str) -> AppResult<String> {
    let value: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| AppError::export(format!("تعذّر قراءة نتيجة التقاط الصورة: {e}")))?;
    Ok(value
        .get("data")
        .and_then(|d| d.as_str())
        .unwrap_or_default()
        .to_string())
}

/// Wait for a channel result without blocking the async runtime.
async fn receive<T: Send + 'static>(rx: mpsc::Receiver<Result<T, String>>) -> AppResult<T> {
    tokio::task::spawn_blocking(move || rx.recv_timeout(OPERATION_TIMEOUT))
        .await
        .map_err(|_| AppError::export("تعذّر تنفيذ عملية التصدير"))?
        .map_err(|_| AppError::export("انتهت مهلة عملية التصدير"))?
        .map_err(AppError::Export)
}

unsafe fn print_to_pdf_inner(
    controller: ICoreWebView2Controller,
    environment: ICoreWebView2Environment,
    tx: mpsc::Sender<Result<Vec<u8>, String>>,
) -> Result<(), String> {
    let core = controller
        .CoreWebView2()
        .map_err(|e| format!("تعذّر الوصول إلى محرك العرض: {e}"))?;
    let webview_16: ICoreWebView2_16 = core
        .cast()
        .map_err(|e| format!("إصدار WebView2 المثبّت لا يدعم إنشاء PDF: {e}"))?;

    // Build A4 print settings with no margins: the document's own `.page`
    // elements already carry the layout.
    let environment6: ICoreWebView2Environment6 = environment
        .cast()
        .map_err(|e| format!("إصدار WebView2 المثبّت لا يدعم إعدادات الطباعة: {e}"))?;
    let settings: ICoreWebView2PrintSettings = environment6
        .CreatePrintSettings()
        .map_err(|e| format!("تعذّر إنشاء إعدادات الطباعة: {e}"))?;
    settings
        .SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT)
        .map_err(|e| format!("تعذّر ضبط اتجاه الصفحة: {e}"))?;
    settings
        .SetPageWidth(A4_WIDTH_INCHES)
        .map_err(|e| format!("تعذّر ضبط عرض الصفحة: {e}"))?;
    settings
        .SetPageHeight(A4_HEIGHT_INCHES)
        .map_err(|e| format!("تعذّر ضبط ارتفاع الصفحة: {e}"))?;
    settings
        .SetMarginTop(0.0)
        .map_err(|e| format!("تعذّر ضبط الهامش: {e}"))?;
    settings
        .SetMarginBottom(0.0)
        .map_err(|e| format!("تعذّر ضبط الهامش: {e}"))?;
    settings
        .SetMarginLeft(0.0)
        .map_err(|e| format!("تعذّر ضبط الهامش: {e}"))?;
    settings
        .SetMarginRight(0.0)
        .map_err(|e| format!("تعذّر ضبط الهامش: {e}"))?;
    settings
        .SetScaleFactor(1.0)
        .map_err(|e| format!("تعذّر ضبط معامل التكبير: {e}"))?;
    settings
        .SetShouldPrintBackgrounds(true)
        .map_err(|e| format!("تعذّر ضبط طباعة الخلفية: {e}"))?;
    settings
        .SetShouldPrintHeaderAndFooter(false)
        .map_err(|e| format!("تعذّر ضبط رؤوس الصفحات: {e}"))?;

    let handler = PrintToPdfStreamCompletedHandler::create(Box::new(move |errorcode, result| {
        if errorcode.is_err() {
            let _ = tx.send(Err(format!("فشل إنشاء ملف PDF (رمز الخطأ {errorcode:?})")));
            return Ok(());
        }
        match result {
            Some(stream) => {
                let outcome = unsafe { read_stream(&stream) };
                let _ = tx.send(outcome);
            }
            None => {
                let _ = tx.send(Err("لم يتم إرجاع بيانات ملف PDF".to_string()));
            }
        }
        Ok(())
    }));

    webview_16
        .PrintToPdfStream(&settings, &handler)
        .map_err(|e| format!("تعذّر بدء إنشاء ملف PDF: {e}"))?;
    Ok(())
}

/// Read every byte from a COM `IStream`.
unsafe fn read_stream(stream: &IStream) -> Result<Vec<u8>, String> {
    // Rewind to the start of the PDF data.
    stream
        .Seek(0, STREAM_SEEK_SET, None)
        .map_err(|e| format!("تعذّر قراءة بيانات PDF: {e}"))?;

    let mut stat = STATSTG::default();
    stream
        .Stat(&mut stat as *mut STATSTG, STATFLAG_DEFAULT)
        .map_err(|e| format!("تعذّر قراءة بيانات PDF: {e}"))?;

    let size = stat.cbSize as usize;
    if size == 0 {
        return Ok(Vec::new());
    }

    let mut buffer = vec![0u8; size];
    let mut read_total = 0usize;
    while read_total < size {
        let remaining = size - read_total;
        let mut read = 0u32;
        let status = stream.Read(
            buffer[read_total..].as_mut_ptr() as *mut core::ffi::c_void,
            remaining.min(u32::MAX as usize) as u32,
            Some(&mut read),
        );
        if status.is_err() {
            break;
        }
        if read == 0 {
            break;
        }
        read_total += read as usize;
    }
    buffer.truncate(read_total);
    Ok(buffer)
}

/// Decode a standard base64 payload (the CDP response format).
fn decode_base64(payload: &str) -> AppResult<Vec<u8>> {
    let cleaned: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    // CDP returns a bare base64 string; strip a data URL prefix if present.
    let encoded = match cleaned.split_once(',') {
        Some((_, rest)) if cleaned.starts_with("data:") => rest.to_string(),
        _ => cleaned,
    };
    decode_base64_bytes(&encoded).ok_or_else(|| AppError::export("تعذّر فك ترميز بيانات الصورة"))
}

fn decode_base64_bytes(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [255u8; 256];
    for (index, byte) in TABLE.iter().enumerate() {
        lookup[*byte as usize] = index as u8;
    }
    lookup[b'-' as usize] = 62; // URL-safe base64
    lookup[b'_' as usize] = 63;

    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for byte in input.bytes() {
        if byte == b'=' {
            break;
        }
        let value = lookup[byte as usize];
        if value == 255 {
            continue;
        }
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xFF) as u8);
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64() {
        assert_eq!(decode_base64_bytes("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64_bytes("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn strips_data_url_prefix() {
        let bytes = decode_base64("data:image/png;base64,aGVsbG8=").unwrap();
        assert_eq!(bytes, b"hello");
    }

    #[test]
    fn parses_a_json_array_result() {
        // `ExecuteScript` returns the JSON encoding of the script result.
        let value =
            parse_json_result("[{\"x\":1.5,\"y\":2.0,\"width\":794.0,\"height\":1123.0}]").unwrap();
        let array = value.as_array().expect("an array");
        assert_eq!(array.len(), 1);
        assert_eq!(array[0].get("width").and_then(|v| v.as_f64()), Some(794.0));
    }

    #[test]
    fn parses_a_double_encoded_json_array_result() {
        // A script that returns `JSON.stringify([...])` is encoded twice.
        let raw = "\"[{\\\"x\\\":1.5,\\\"width\\\":794.0}]\"";
        let value = parse_json_result(raw).unwrap();
        let array = value.as_array().expect("an array");
        assert_eq!(array.len(), 1);
    }

    #[test]
    fn parses_a_primitive_result() {
        assert_eq!(parse_json_result("true").unwrap().as_bool(), Some(true));
        assert!(parse_json_result("undefined").unwrap().is_null());
        assert!(parse_json_result("").unwrap().is_null());
    }
}
