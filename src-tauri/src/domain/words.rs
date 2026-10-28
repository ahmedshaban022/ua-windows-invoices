//! Writing monetary amounts out in Arabic words.
//!
//! Egyptian invoices traditionally carry the total in words next to the
//! numeric total. The wording follows the Egyptian business convention
//! (dual/plural forms for 3–10 and 11–99).

use crate::domain::money::Currency;

/// Render `minor` units as an Arabic phrase, e.g.
/// "ألف ومائتان وخمسون جنيهاً وخمسون قرشًا".
pub fn amount_in_words(minor: i64, currency: &Currency) -> String {
    let scale = 10i64.saturating_pow(currency.exponent).max(1);
    let negative = minor < 0;
    let abs = minor.unsigned_abs();
    let major = abs / scale as u64;
    let frac = abs % scale as u64;

    let mut parts: Vec<String> = Vec::new();
    if major > 0 {
        parts.push(format!(
            "{} {}",
            integer_to_words(major),
            currency.major_unit_ar
        ));
    }
    if frac > 0 {
        let mut phrase = format!("{} {}", integer_to_words(frac), currency.minor_unit_ar);
        if major > 0 {
            phrase = format!("و{}", phrase);
        }
        parts.push(phrase);
    }

    if parts.is_empty() {
        return format!("صفر {}", currency.major_unit_ar);
    }

    let mut out = parts.join(" و");
    if negative {
        out = format!("سالب {}", out);
    }
    out
}

/// Convert a non-negative integer into Arabic words.
pub fn integer_to_words(value: u64) -> String {
    if value == 0 {
        return "صفر".to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    let mut rest = value;

    let millions = rest / 1_000_000;
    rest %= 1_000_000;
    if millions > 0 {
        parts.push(scale_word(millions, "مليون", "مليونان", "ملايين"));
    }

    let thousands = rest / 1_000;
    rest %= 1_000;
    if thousands > 0 {
        parts.push(scale_word(thousands, "ألف", "ألفان", "آلاف"));
    }

    if rest > 0 {
        parts.push(under_thousand(rest));
    }

    parts.join(" و")
}

/// Word for a scaled group: 1 => "ألف", 2 => "ألفان", 3..10 => "ثلاثة آلاف",
/// 11..99 => "ثلاثة عشر ألفًا".
fn scale_word(count: u64, one: &str, two: &str, plural: &str) -> String {
    match count {
        1 => one.to_string(),
        2 => two.to_string(),
        3..=10 => format!("{} {}", under_thousand(count), plural),
        11..=99 => format!("{} {}ًا", under_thousand(count), one.trim_end_matches("ان")),
        _ => format!("{} {}", under_thousand(count), one.trim_end_matches("ان")),
    }
}

/// Words for a number below 1000.
fn under_thousand(value: u64) -> String {
    let hundreds = value / 100;
    let rest = value % 100;
    let mut parts: Vec<String> = Vec::new();

    if hundreds > 0 {
        parts.push(match hundreds {
            1 => "مائة".to_string(),
            2 => "مائتان".to_string(),
            3..=10 => format!("{}مائة", ones(hundreds)),
            _ => format!("{}مائة", under_hundred(hundreds)),
        });
    }
    if rest > 0 {
        parts.push(under_hundred(rest));
    }
    parts.join(" و")
}

/// Words for a number below 100.
fn under_hundred(value: u64) -> String {
    if value < 20 {
        return tens_and_teens(value).to_string();
    }
    let tens = value / 10;
    let ones = value % 10;
    let tens_word = match tens {
        2 => "عشرون",
        3 => "ثلاثون",
        4 => "أربعون",
        5 => "خمسون",
        6 => "ستون",
        7 => "سبعون",
        8 => "ثمانون",
        9 => "تسعون",
        _ => "",
    };
    if ones == 0 {
        tens_word.to_string()
    } else {
        format!("{} و{}", ones_word(ones), tens_word)
    }
}

fn tens_and_teens(value: u64) -> &'static str {
    match value {
        0 => "صفر",
        1 => "واحد",
        2 => "اثنان",
        3 => "ثلاثة",
        4 => "أربعة",
        5 => "خمسة",
        6 => "ستة",
        7 => "سبعة",
        8 => "ثمانية",
        9 => "تسعة",
        10 => "عشرة",
        11 => "أحد عشر",
        12 => "اثنا عشر",
        13 => "ثلاثة عشر",
        14 => "أربعة عشر",
        15 => "خمسة عشر",
        16 => "ستة عشر",
        17 => "سبعة عشر",
        18 => "ثمانية عشر",
        19 => "تسعة عشر",
        _ => "",
    }
}

/// Accusative form used in "ثلاثة آلاف" (3–10).
fn ones(value: u64) -> &'static str {
    match value {
        3 => "ثلاثة",
        4 => "أربعة",
        5 => "خمسة",
        6 => "ستة",
        7 => "سبعة",
        8 => "ثمانية",
        9 => "تسعة",
        10 => "عشرة",
        _ => "",
    }
}

/// Accusative singular used in "ثلاثة عشر".
fn ones_word(value: u64) -> &'static str {
    match value {
        1 => "واحدًا",
        2 => "اثنان",
        3 => "ثلاثة",
        4 => "أربعة",
        5 => "خمسة",
        6 => "ستة",
        7 => "سبعة",
        8 => "ثمانية",
        9 => "تسعة",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::Currency;

    #[test]
    fn writes_simple_numbers() {
        assert_eq!(integer_to_words(0), "صفر");
        assert_eq!(integer_to_words(1), "واحد");
        assert_eq!(integer_to_words(3), "ثلاثة");
        assert_eq!(integer_to_words(15), "خمسة عشر");
        assert_eq!(integer_to_words(21), "واحدًا وعشرون");
        assert_eq!(integer_to_words(100), "مائة");
        assert_eq!(integer_to_words(200), "مائتان");
    }

    #[test]
    fn writes_thousands_and_millions() {
        assert_eq!(integer_to_words(1000), "ألف");
        assert_eq!(integer_to_words(2000), "ألفان");
        assert_eq!(integer_to_words(3000), "ثلاثة آلاف");
        assert_eq!(integer_to_words(12000), "اثنا عشر ألفًا");
        assert_eq!(integer_to_words(1_000_000), "مليون");
        assert_eq!(integer_to_words(2_000_000), "مليونان");
    }

    #[test]
    fn writes_currency_amounts() {
        let egp = Currency::egp();
        let text = amount_in_words(125_050, &egp);
        assert!(text.contains("جنيهاً"), "{text}");
        assert!(text.contains("قرشًا"), "{text}");
        assert_eq!(amount_in_words(0, &egp), "صفر جنيهاً");
    }
}
