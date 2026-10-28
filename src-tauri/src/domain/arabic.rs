//! Arabic text normalization for **search only** (FR-02).
//!
//! The original product name is always preserved for display; this normalized
//! form exists purely so that "أ", "إ" and "آ" match, and so that diacritics and
//! tatweel do not break a search.

/// Normalize an Arabic string for search matching.
///
/// - hamza family -> base alef
/// - alef maqsura -> yeh, teh marbuta -> heh, waw hamza -> waw, yeh hamza -> yeh
/// - strip tashkeel (harakat) and tatweel
/// - convert Arabic-Indic digits to ASCII
/// - collapse whitespace, trim, lowercase
pub fn normalize_search(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        let mapped = map_char(ch);
        if mapped.is_none() {
            continue; // diacritic / tatweel / other combining marks are dropped
        }
        let mapped = mapped.unwrap();
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.extend(mapped.to_lowercase());
    }
    out
}

/// Map a single character to its normalized form, or `None` when it should be
/// removed entirely (tashkeel, tatweel).
fn map_char(ch: char) -> Option<char> {
    let mapped = match ch {
        // hamza family -> alef
        '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' | '\u{0672}' | '\u{0673}' => '\u{0627}',
        // alef maqsura -> yeh
        '\u{0649}' => '\u{064A}',
        // teh marbuta -> heh
        '\u{0629}' => '\u{0647}',
        // waw with hamza -> waw
        '\u{0624}' => '\u{0648}',
        // yeh with hamza -> yeh
        '\u{0626}' => '\u{064A}',
        // Arabic-Indic digits -> ASCII
        '\u{0660}'..='\u{0669}' => char::from_digit(ch as u32 - 0x0660, 10).unwrap(),
        '\u{06F0}'..='\u{06F9}' => char::from_digit(ch as u32 - 0x06F0, 10).unwrap(),
        // tashkeel / harakat / superscript alef / quranic marks: drop
        '\u{064B}'..='\u{0652}'
        | '\u{0670}'
        | '\u{0610}'..='\u{061A}'
        | '\u{06D6}'..='\u{06ED}'
        | '\u{0640}' => return None,
        other => other,
    };
    Some(mapped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_hamza_variants() {
        assert_eq!(normalize_search("أحمد"), normalize_search("احمد"));
        assert_eq!(normalize_search("إبراهيم"), normalize_search("ابراهيم"));
        assert_eq!(normalize_search("آمنة"), normalize_search("امنة"));
    }

    #[test]
    fn normalizes_letter_shape_variants() {
        assert_eq!(normalize_search("مويزة"), normalize_search("مويزة"));
        assert_eq!(normalize_search("عربية"), normalize_search("عربيه"));
        assert_eq!(normalize_search("مسؤول"), normalize_search("مسوول"));
    }

    #[test]
    fn strips_diacritics_and_tatweel() {
        assert_eq!(normalize_search("مُحَمَّد"), "محمد");
        assert_eq!(normalize_search("مـــحمد"), "محمد");
    }

    #[test]
    fn converts_arabic_indic_digits() {
        assert_eq!(normalize_search("المنتج ٢٥"), "المنتج 25");
    }

    #[test]
    fn collapses_whitespace_and_lowercases() {
        assert_eq!(normalize_search("  حليب   كامل  "), "حليب كامل");
        assert_eq!(normalize_search("Milk"), "milk");
    }

    #[test]
    fn preserves_original_for_display() {
        let original = "أ.Product";
        let normalized = normalize_search(original);
        assert_ne!(original, normalized);
        assert_eq!(normalized, "ا.product");
    }
}
