//! WebView2 export pipeline (FR-07).
//!
//! PDF, print and PNG/JPG all come from **one** canonical invoice document that
//! lives in a dedicated, chrome-free "render host" webview. Chromium is the
//! renderer, so Arabic shaping, RTL, inline SVG fidelity, A4 pagination and
//! repeated table headers are all native behaviour rather than something we
//! re-implement (and therefore can never drift from the preview).
//!
//! Only one print job may run per webview at a time, so exports are serialized
//! behind a mutex.

use serde::{Deserialize, Serialize};

use crate::domain::error::{AppError, AppResult};

#[cfg(windows)]
pub mod windows;

/// A4 dimensions in inches (210mm x 297mm).
pub const A4_WIDTH_INCHES: f64 = 210.0 / 25.4;
pub const A4_HEIGHT_INCHES: f64 = 297.0 / 25.4;

/// The render host is sized to A4 at 96 dpi.
pub const RENDER_HOST_WIDTH: f64 = 794.0;
pub const RENDER_HOST_HEIGHT: f64 = 1123.0;

/// Serializes export jobs: WebView2 allows only one print job per webview.
///
/// A `tokio` semaphore is used (rather than `std`) so the permit can be held
/// across `await` points while keeping the command future `Send`.
static EXPORT_LOCK: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

/// Acquires the export lock, returning a permit.
pub async fn acquire_export_lock() -> AppResult<tokio::sync::SemaphorePermit<'static>> {
    EXPORT_LOCK
        .acquire()
        .await
        .map_err(|_| AppError::export("تعذّر بدء عملية التصدير"))
}

/// The output format for an image export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageFormat {
    Png,
    Jpg,
}

impl ImageFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpg => "jpg",
        }
    }
}

/// The result of an image export: one file per invoice page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageExportResult {
    /// Absolute paths of the written files, in page order.
    pub files: Vec<String>,
    pub page_count: usize,
}

/// A page rectangle reported by the render host, in CSS pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Whether a captured image is effectively blank.
///
/// Chromium does not always paint a never-shown window; a blank capture is
/// detected here and the caller falls back to showing the render host before
/// capturing again (PRD risk mitigation, §6.7 of the plan).
pub fn is_blank_png(bytes: &[u8]) -> bool {
    let image = match image::load_from_memory(bytes) {
        Ok(image) => image.to_rgba8(),
        Err(_) => return true,
    };
    // Sample a grid of pixels; if every sampled pixel is (near) white, the
    // capture carries no content.
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 {
        return true;
    }
    let mut non_white = 0u32;
    let samples_x = 16u32;
    let samples_y = 16u32;
    for sy in 0..samples_y {
        for sx in 0..samples_x {
            let x = ((sx as u64 + 1) * width as u64 / (samples_x as u64 + 1)) as u32;
            let y = ((sy as u64 + 1) * height as u64 / (samples_y as u64 + 1)) as u32;
            let x = x.min(width - 1);
            let y = y.min(height - 1);
            let pixel = image.get_pixel(x, y);
            let alpha = pixel[3];
            // Transparent pixels are treated as blank background too.
            if alpha > 16 && (pixel[0] < 245 || pixel[1] < 245 || pixel[2] < 245) {
                non_white += 1;
            }
        }
    }
    non_white == 0
}

/// Flatten a PNG onto an opaque white background and encode it as JPEG.
///
/// JPG has no alpha channel, so PNG transparency must never leak through as
/// black (FR-07: "JPG must have an opaque background").
pub fn png_to_opaque_jpg(png: &[u8]) -> AppResult<Vec<u8>> {
    let image = image::load_from_memory(png)
        .map_err(|e| AppError::export(format!("تعذّر قراءة الصورة: {e}")))?
        .to_rgba8();

    let mut rgb = image::RgbImage::new(image.width(), image.height());
    for (x, y, pixel) in image.enumerate_pixels() {
        let alpha = pixel[3] as f32 / 255.0;
        // Composite over white.
        let r = (pixel[0] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
        let g = (pixel[1] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
        let b = (pixel[2] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
        rgb.put_pixel(x, y, image::Rgb([r, g, b]));
    }

    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| AppError::export(format!("تعذّر إنشاء ملف JPG: {e}")))?;
    Ok(out)
}

/// Build the file name for an exported page image.
pub fn page_file_name(invoice_number: &str, page_index: usize, format: ImageFormat) -> String {
    let safe = invoice_number
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{safe}-p{page_index}.{}", format.extension())
}

/// Derive the PDF file name for an invoice.
pub fn pdf_file_name(invoice_number: &str) -> String {
    let safe = invoice_number
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{safe}.pdf")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_dimensions_are_correct() {
        assert!((A4_WIDTH_INCHES - 8.2677).abs() < 0.001);
        assert!((A4_HEIGHT_INCHES - 11.6929).abs() < 0.001);
    }

    #[test]
    fn blank_png_is_detected() {
        // A fully white PNG.
        let mut white = image::RgbImage::new(20, 20);
        for pixel in white.pixels_mut() {
            *pixel = image::Rgb([255, 255, 255]);
        }
        let mut buffer = Vec::new();
        image::DynamicImage::ImageRgb8(white)
            .write_to(
                &mut std::io::Cursor::new(&mut buffer),
                image::ImageFormat::Png,
            )
            .unwrap();
        assert!(is_blank_png(&buffer));
    }

    #[test]
    fn non_blank_png_is_not_flagged() {
        let mut img = image::RgbaImage::new(20, 20);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 255]);
        }
        let mut buffer = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut buffer),
                image::ImageFormat::Png,
            )
            .unwrap();
        assert!(!is_blank_png(&buffer));
    }

    #[test]
    fn png_becomes_opaque_jpg() {
        let mut img = image::RgbaImage::new(10, 10);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([255, 0, 0, 0]); // fully transparent red
        }
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();

        let jpg = png_to_opaque_jpg(&png).unwrap();
        let decoded = image::load_from_memory(&jpg).unwrap().to_rgb8();
        let pixel = decoded.get_pixel(5, 5);
        // Transparent red over white must become white, not black.
        assert!(pixel[0] > 245 && pixel[1] > 245 && pixel[2] > 245);
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(pdf_file_name("INV-000001"), "INV-000001.pdf");
        assert_eq!(
            page_file_name("INV-000001", 2, ImageFormat::Png),
            "INV-000001-p2.png"
        );
        assert_eq!(
            page_file_name("INV/0001", 1, ImageFormat::Jpg),
            "INV_0001-p1.jpg"
        );
    }
}
