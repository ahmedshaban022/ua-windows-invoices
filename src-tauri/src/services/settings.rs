//! Settings service (FR-10).
//!
//! Settings live in the `app_settings` key/value table as JSON. The logo is
//! stored separately as an SVG file under the brand directory.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::now;
use crate::domain::error::{AppError, AppResult};
use crate::domain::money::Currency;

/// The full settings block shown in the Settings screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub company_name: String,
    pub company_phone: String,
    pub company_address: String,
    pub logo_svg: String,
    pub brand_color: String,
    pub currency_code: String,
    pub currency_exponent: u32,
    pub date_format: String,
    pub backup_location: String,
    pub auto_backup_enabled: bool,
    pub auto_backup_retention: i64,
    /// The custom terms (بنود) the user can attach to an invoice.
    #[serde(default)]
    pub invoice_terms: Vec<String>,
}

/// The values older installs stored as placeholders. They are treated as
/// "not filled in" so a placeholder never reaches a printed invoice.
const PLACEHOLDER_COMPANY_NAME: &str = "اسم الشركة";
const PLACEHOLDER_COMPANY_PHONE: &str = "رقم الهاتف";
const PLACEHOLDER_COMPANY_ADDRESS: &str = "العنوان";

/// Drop a placeholder value: an empty or never-edited field stays empty.
fn clean(value: &str, placeholder: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == placeholder {
        String::new()
    } else {
        trimmed.to_string()
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            company_name: String::new(),
            company_phone: String::new(),
            company_address: String::new(),
            logo_svg: include_str!("../../../assets/brand/logo.svg").to_string(),
            brand_color: "#0b68e6".to_string(),
            currency_code: "EGP".to_string(),
            currency_exponent: 2,
            date_format: "dd/MM/yyyy".to_string(),
            backup_location: String::new(),
            auto_backup_enabled: false,
            auto_backup_retention: 7,
            invoice_terms: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn currency(&self) -> Currency {
        Currency::by_exponent(&self.currency_code, self.currency_exponent)
    }

    /// The company name to show, or `None` when it was never filled in.
    pub fn display_company_name(&self) -> Option<&str> {
        let name = self.company_name.trim();
        if name.is_empty() || name == PLACEHOLDER_COMPANY_NAME {
            None
        } else {
            Some(name)
        }
    }

    /// The seller identity without the placeholder values.
    pub fn clean_company(&self) -> (String, String, String) {
        (
            clean(&self.company_name, PLACEHOLDER_COMPANY_NAME),
            clean(&self.company_phone, PLACEHOLDER_COMPANY_PHONE),
            clean(&self.company_address, PLACEHOLDER_COMPANY_ADDRESS),
        )
    }

    /// The custom terms, without blank entries.
    pub fn clean_terms(&self) -> Vec<String> {
        self.invoice_terms
            .iter()
            .map(|term| term.trim().to_string())
            .filter(|term| !term.is_empty())
            .collect()
    }

    /// Validate a brand color: `#rrggbb` (case-insensitive).
    pub fn validate_brand_color(color: &str) -> AppResult<String> {
        let trimmed = color.trim();
        let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::validation(
                "اللون يجب أن يكون بصيغة ست عشري مثل #0b68e6",
            ));
        }
        Ok(format!("#{}", hex.to_ascii_lowercase()))
    }
}

/// Load settings, falling back to defaults for missing keys.
pub fn load(conn: &Connection) -> AppResult<AppSettings> {
    let mut settings = AppSettings::default();
    for (key, value) in load_all(conn)? {
        let text = value.as_str().unwrap_or_default().to_string();
        match key.as_str() {
            "company_name" => settings.company_name = text,
            "company_phone" => settings.company_phone = text,
            "company_address" => settings.company_address = text,
            "invoice_terms" => {
                if let Some(list) = value.as_array() {
                    settings.invoice_terms = list
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|item| item.to_string())
                        .collect();
                }
            }
            "logo_svg" => settings.logo_svg = text,
            "brand_color" => settings.brand_color = text,
            "currency_code" => settings.currency_code = text,
            "currency_exponent" => {
                if let Some(number) = value.as_u64() {
                    settings.currency_exponent = number as u32;
                }
            }
            "date_format" => settings.date_format = text,
            "backup_location" => settings.backup_location = text,
            "auto_backup_enabled" => {
                if let Some(flag) = value.as_bool() {
                    settings.auto_backup_enabled = flag;
                }
            }
            "auto_backup_retention" => {
                if let Some(number) = value.as_u64() {
                    settings.auto_backup_retention = number as i64;
                }
            }
            _ => {}
        }
    }
    Ok(settings)
}

fn load_all(conn: &Connection) -> AppResult<Vec<(String, serde_json::Value)>> {
    let mut stmt = conn.prepare("SELECT key, value_json FROM app_settings")?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let raw: String = row.get(1)?;
        Ok((key, raw))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (key, raw) = row?;
        let value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
        out.push((key, value));
    }
    Ok(out)
}

/// Persist the settings block.
pub fn save(conn: &Connection, settings: &AppSettings) -> AppResult<AppSettings> {
    let brand_color = AppSettings::validate_brand_color(&settings.brand_color)?;
    let mut settings = settings.clone();
    settings.brand_color = brand_color;

    let tx = conn.unchecked_transaction()?;
    let entries: Vec<(&str, serde_json::Value)> = vec![
        ("company_name", settings.company_name.clone().into()),
        ("company_phone", settings.company_phone.clone().into()),
        ("company_address", settings.company_address.clone().into()),
        (
            "invoice_terms",
            serde_json::Value::Array(
                settings
                    .invoice_terms
                    .iter()
                    .map(|term| serde_json::Value::String(term.clone()))
                    .collect(),
            ),
        ),
        ("logo_svg", settings.logo_svg.clone().into()),
        ("brand_color", settings.brand_color.clone().into()),
        ("currency_code", settings.currency_code.clone().into()),
        ("currency_exponent", settings.currency_exponent.into()),
        ("date_format", settings.date_format.clone().into()),
        ("backup_location", settings.backup_location.clone().into()),
        ("auto_backup_enabled", settings.auto_backup_enabled.into()),
        (
            "auto_backup_retention",
            settings.auto_backup_retention.into(),
        ),
    ];

    for (key, value) in entries {
        tx.execute(
            "INSERT INTO app_settings(key, value_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
            params![key, serde_json::to_string(&value)?, now()],
        )?;
    }
    tx.commit()?;
    Ok(settings)
}

/// Read a single raw setting value.
pub fn get_value(conn: &Connection, key: &str) -> AppResult<Option<serde_json::Value>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    match raw {
        Some(text) => Ok(Some(
            serde_json::from_str(&text).unwrap_or(serde_json::Value::Null),
        )),
        None => Ok(None),
    }
}

/// Write a single raw setting value.
pub fn set_value(conn: &Connection, key: &str, value: &serde_json::Value) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings(key, value_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        params![key, serde_json::to_string(value)?, now()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn defaults_are_arabic_placeholders() {
        let settings = AppSettings::default();
        // The company identity starts empty: a placeholder must never end up
        // on a printed invoice.
        assert_eq!(settings.company_name, "");
        assert_eq!(settings.display_company_name(), None);
        assert_eq!(settings.brand_color, "#0b68e6");
        assert_eq!(settings.currency_code, "EGP");
        assert!(settings.logo_svg.contains("<svg"));
    }

    #[test]
    fn a_placeholder_company_name_counts_as_not_set() {
        let settings = AppSettings {
            company_name: "اسم الشركة".to_string(),
            company_phone: "رقم الهاتف".to_string(),
            company_address: "العنوان".to_string(),
            ..AppSettings::default()
        };
        assert_eq!(settings.display_company_name(), None);
        assert_eq!(
            settings.clean_company(),
            (String::new(), String::new(), String::new())
        );
    }

    #[test]
    fn a_real_company_name_is_kept() {
        let settings = AppSettings {
            company_name: "  شركة النور  ".to_string(),
            ..AppSettings::default()
        };
        assert_eq!(settings.display_company_name(), Some("شركة النور"));
        assert_eq!(settings.clean_company().0, "شركة النور");
    }

    #[test]
    fn invoice_terms_round_trip() {
        let conn = open_in_memory().unwrap();
        let settings = AppSettings {
            invoice_terms: vec![
                "حقوق النشر محفوظة".to_string(),
                "ضمان شركة الأبنودي لمدة ثلاث أعوام".to_string(),
            ],
            ..AppSettings::default()
        };
        save(&conn, &settings).unwrap();

        let loaded = load(&conn).unwrap();
        assert_eq!(loaded.invoice_terms.len(), 2);
        assert_eq!(loaded.invoice_terms[0], "حقوق النشر محفوظة");
    }

    #[test]
    fn blank_terms_are_dropped() {
        let settings = AppSettings {
            invoice_terms: vec!["  ".to_string(), " garanty ".to_string()],
            ..AppSettings::default()
        };
        assert_eq!(settings.clean_terms(), vec!["garanty".to_string()]);
    }

    #[test]
    fn settings_round_trip() {
        let conn = open_in_memory().unwrap();
        let settings = AppSettings {
            company_name: "شركة النور".to_string(),
            brand_color: "#0B68E6".to_string(),
            ..AppSettings::default()
        };
        let saved = save(&conn, &settings).unwrap();
        assert_eq!(saved.brand_color, "#0b68e6");

        let loaded = load(&conn).unwrap();
        assert_eq!(loaded.company_name, "شركة النور");
        assert_eq!(loaded.brand_color, "#0b68e6");
        assert_eq!(loaded.currency_code, "EGP");
    }

    #[test]
    fn rejects_invalid_brand_color() {
        let conn = open_in_memory().unwrap();
        let settings = AppSettings {
            brand_color: "blue".to_string(),
            ..AppSettings::default()
        };
        assert!(save(&conn, &settings).is_err());
    }
}
