//! Brand service: the logo asset and the seller snapshot (FR-10).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::db::AppPaths;
use crate::domain::error::{AppError, AppResult};
use crate::domain::invoice::SellerSnapshot;
use crate::services::settings::AppSettings;

/// The brand logo, stored as inline SVG markup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandLogo {
    /// The raw SVG markup.
    pub svg: String,
    /// The brand color applied to the logo.
    pub brand_color: String,
}

/// The default logo shipped with the application.
pub const DEFAULT_LOGO_SVG: &str = include_str!("../../../assets/brand/logo.svg");

/// Read the stored logo from the brand directory, falling back to the built-in one.
pub fn load_logo(paths: &AppPaths) -> String {
    let file = paths.logo_file();
    match std::fs::read_to_string(&file) {
        Ok(contents) if contents.trim_start().starts_with("<svg") => contents,
        _ => DEFAULT_LOGO_SVG.to_string(),
    }
}

/// Store the logo on disk. The SVG is validated as an SVG before being saved.
pub fn save_logo(paths: &AppPaths, svg: &str) -> AppResult<()> {
    let trimmed = svg.trim();
    if !trimmed.starts_with("<svg") || !trimmed.contains("</svg>") {
        return Err(AppError::validation("الملف المختار ليس صورة SVG صالحة"));
    }
    // A minimal safety check: reject anything that could carry script.
    let lowered = trimmed.to_ascii_lowercase();
    for forbidden in ["<script", "onload=", "onerror=", "javascript:"] {
        if lowered.contains(forbidden) {
            return Err(AppError::validation(
                "ملف الشعار يحتوي على عناصر غير مسموح بها",
            ));
        }
    }
    std::fs::create_dir_all(&paths.brand)
        .map_err(|e| AppError::storage(format!("تعذّر إنشاء مجلد الشعار: {e}")))?;
    std::fs::write(paths.logo_file(), trimmed)
        .map_err(|e| AppError::storage(format!("تعذّر حفظ الشعار: {e}")))?;
    Ok(())
}

/// Build the seller snapshot captured when an invoice is issued.
pub fn snapshot(settings: &AppSettings, logo_svg: &str) -> SellerSnapshot {
    // A company name that was never filled in stays empty, so it is never
    // printed on a document.
    let (company_name, company_phone, company_address) = settings.clean_company();
    SellerSnapshot {
        company_name,
        company_phone,
        company_address,
        logo_svg: logo_svg.to_string(),
        brand_color: settings.brand_color.clone(),
        currency_code: settings.currency_code.clone(),
        currency_exponent: settings.currency_exponent,
        date_format: settings.date_format.clone(),
    }
}

/// Recolor an SVG by replacing its fill/stroke color with the brand color.
/// Used when the owner picks a new brand color in Settings.
pub fn recolor_svg(svg: &str, brand_color: &str) -> String {
    let color = brand_color.trim();
    // Replace every #rrggbb literal with the brand color, preserving the rest
    // of the artwork exactly.
    let mut out = String::with_capacity(svg.len());
    let bytes: Vec<char> = svg.chars().collect();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == '#'
            && index + 6 < bytes.len()
            && bytes[index + 1..index + 7]
                .iter()
                .all(|c| c.is_ascii_hexdigit())
        {
            out.push_str(color);
            index += 7;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    out
}

/// Whether the given file looks like an SVG document.
pub fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("svg"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recolors_svg_preserving_geometry() {
        let svg = r##"<svg viewBox="0 0 10 10"><path d="M0 0L10 10" fill="#0F6EFE"/></svg>"##;
        let out = recolor_svg(svg, "#0b68e6");
        assert!(out.contains("#0b68e6"));
        assert!(!out.contains("#0F6EFE"));
        // The path geometry is untouched.
        assert!(out.contains(r#"d="M0 0L10 10""#));
    }

    #[test]
    fn rejects_unsafe_svg() {
        assert!(save_logo(
            &AppPaths::resolve("com.ua.invoice.unsafe-test").unwrap(),
            "<svg><script>alert(1)</script></svg>"
        )
        .is_err());
    }
}
