//! Money handling.
//!
//! Every monetary amount is an `i64` count of **minor units** (piastres for EGP).
//! Binary floating point is never used for money anywhere in this application
//! (PRD §7). Formatting happens only at the edges, using the currency's
//! minor-unit exponent.

use serde::{Deserialize, Serialize};

/// A currency, identified by ISO code plus its minor-unit exponent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Currency {
    pub code: String,
    /// Number of decimal places. EGP => 2 (1 EGP = 100 piastres).
    pub exponent: u32,
    /// Arabic display name, e.g. "جنيه مصري".
    pub name_ar: String,
    /// Arabic name of one major unit, e.g. "جنيه".
    pub major_unit_ar: String,
    /// Arabic name of one minor unit, e.g. "قرش".
    pub minor_unit_ar: String,
}

impl Currency {
    pub fn egp() -> Self {
        Self {
            code: "EGP".into(),
            exponent: 2,
            name_ar: "جنيه مصري".into(),
            major_unit_ar: "جنيهاً".into(),
            minor_unit_ar: "قرشًا".into(),
        }
    }

    pub fn usd() -> Self {
        Self {
            code: "USD".into(),
            exponent: 2,
            name_ar: "دولار أمريكي".into(),
            major_unit_ar: "دولارًا".into(),
            minor_unit_ar: "سنتًا".into(),
        }
    }

    pub fn eur() -> Self {
        Self {
            code: "EUR".into(),
            exponent: 2,
            name_ar: "أورو".into(),
            major_unit_ar: "يورو".into(),
            minor_unit_ar: "سنتًا".into(),
        }
    }

    pub fn sar() -> Self {
        Self {
            code: "SAR".into(),
            exponent: 2,
            name_ar: "ريال سعودي".into(),
            major_unit_ar: "ريالًا".into(),
            minor_unit_ar: "هللة".into(),
        }
    }

    pub fn aed() -> Self {
        Self {
            code: "AED".into(),
            exponent: 2,
            name_ar: "درهم إماراتي".into(),
            major_unit_ar: "درهمًا".into(),
            minor_unit_ar: "فلسًا".into(),
        }
    }

    pub fn by_exponent(code: &str, exponent: u32) -> Self {
        match code.to_ascii_uppercase().as_str() {
            "EGP" => Currency::egp(),
            "USD" => Currency::usd(),
            "EUR" => Currency::eur(),
            "SAR" => Currency::sar(),
            "AED" => Currency::aed(),
            _ => Self {
                code: code.to_ascii_uppercase(),
                exponent,
                name_ar: code.to_ascii_uppercase(),
                major_unit_ar: code.to_ascii_uppercase(),
                minor_unit_ar: "وحدة فرعية".into(),
            },
        }
    }

    /// Convert a major-unit decimal string ("12.50") into minor units.
    /// Returns `None` when the text is not a valid non-negative decimal.
    pub fn parse_major_to_minor(&self, text: &str) -> Option<i64> {
        // The Arabic decimal separator (U+066B) is the decimal mark. A comma is
        // treated as a decimal mark too when the text has no dot ("12,50"),
        // and as a thousands separator otherwise ("1,234.50").
        let trimmed = text.trim();
        let mut cleaned = trimmed.replace('\u{066B}', ".");
        if cleaned.contains('.') {
            cleaned = cleaned.replace(',', "");
        } else if cleaned.matches(',').count() == 1 {
            cleaned = cleaned.replace(',', ".");
        } else {
            cleaned = cleaned.replace(',', "");
        }
        let cleaned: String = cleaned.chars().filter(|c| !c.is_whitespace()).collect();
        if cleaned.is_empty() {
            return None;
        }
        let (int_part, frac_part) = match cleaned.split_once('.') {
            Some((a, b)) => (a, b),
            None => (cleaned.as_str(), ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        if !int_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if !frac_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let int_value: i64 = if int_part.is_empty() {
            0
        } else {
            int_part.parse().ok()?
        };
        // Round half-up at the currency's precision: extra decimals are
        // truncated after the first dropped digit is compared against 5, so
        // 12.999 with exponent 2 becomes 1300 minor units.
        let scale = 10i64.checked_pow(self.exponent)?;
        let mut minor = int_value.checked_mul(scale)?;

        let mut frac_value: i64 = 0;
        let mut digits: u32 = 0;
        let mut round_up = false;
        for (index, ch) in frac_part.chars().enumerate() {
            let digit = ch.to_digit(10)? as i64;
            if (index as u32) < self.exponent {
                frac_value = frac_value * 10 + digit;
                digits += 1;
            } else {
                // The first dropped digit decides the rounding (half-up).
                round_up = digit >= 5;
                break;
            }
        }
        frac_value = frac_value.checked_mul(10i64.checked_pow(self.exponent - digits)?)?;
        minor = minor.checked_add(frac_value)?;
        if round_up {
            minor = minor.checked_add(1)?;
        }
        Some(minor)
    }

    /// Format minor units as a major-unit decimal string, e.g. 1250 => "12.50".
    pub fn format_minor(&self, minor: i64) -> String {
        let scale = 10i64.saturating_pow(self.exponent);
        let negative = minor < 0;
        let abs = minor.unsigned_abs();
        let major = abs / scale as u64;
        let frac = abs % scale as u64;
        let sign = if negative { "-" } else { "" };
        if self.exponent == 0 {
            return format!("{sign}{major}");
        }
        let frac_str = format!("{frac:0width$}", width = self.exponent as usize);
        format!("{sign}{major}.{frac_str}")
    }
}

/// Parse a price typed by the user into minor units.
/// Accepts Arabic-Indic digits and both "." and "٫" as the decimal separator.
pub fn parse_price(text: &str, currency: &Currency) -> Option<i64> {
    let normalized = normalize_digits(text);
    currency.parse_major_to_minor(&normalized)
}

/// Convert Arabic-Indic and Eastern Arabic-Indic digits to ASCII digits.
pub fn normalize_digits(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{0660}'..='\u{0669}' => char::from_digit(c as u32 - 0x0660, 10).unwrap(),
            '\u{06F0}'..='\u{06F9}' => char::from_digit(c as u32 - 0x06F0, 10).unwrap(),
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_major_to_minor() {
        let egp = Currency::egp();
        assert_eq!(egp.parse_major_to_minor("12.50"), Some(1250));
        assert_eq!(egp.parse_major_to_minor("12"), Some(1200));
        assert_eq!(egp.parse_major_to_minor("0.05"), Some(5));
        assert_eq!(egp.parse_major_to_minor(".5"), Some(50));
        assert_eq!(egp.parse_major_to_minor("  7.25 "), Some(725));
    }

    #[test]
    fn rejects_malformed_prices() {
        let egp = Currency::egp();
        assert_eq!(egp.parse_major_to_minor(""), None);
        assert_eq!(egp.parse_major_to_minor("abc"), None);
        assert_eq!(egp.parse_major_to_minor("-5"), None);
        assert_eq!(egp.parse_major_to_minor("1.2.3"), None);
    }

    #[test]
    fn rounds_extra_decimals_half_up() {
        let egp = Currency::egp();
        assert_eq!(egp.parse_major_to_minor("12.999"), Some(1300));
        assert_eq!(egp.parse_major_to_minor("12.994"), Some(1299));
    }

    #[test]
    fn formats_minor_back_to_major() {
        let egp = Currency::egp();
        assert_eq!(egp.format_minor(1250), "12.50");
        assert_eq!(egp.format_minor(5), "0.05");
        assert_eq!(egp.format_minor(1200), "12.00");
        assert_eq!(egp.format_minor(0), "0.00");
    }

    #[test]
    fn accepts_arabic_indic_digits_and_separator() {
        let egp = Currency::egp();
        assert_eq!(parse_price("١٢٫٥٠", &egp), Some(1250));
        assert_eq!(parse_price("12,50", &egp), Some(1250));
        assert_eq!(parse_price("1,234.50", &egp), Some(123_450));
    }
}
