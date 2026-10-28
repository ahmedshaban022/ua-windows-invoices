//! Invoice numbering (FR-01).
//!
//! Numbers are consumed from a durable counter **inside the same transaction**
//! as the invoice insert, so a number can never be issued twice and is never
//! recycled — not even after the invoice is deleted.

/// Format a sequence value as an invoice number, e.g. 1 -> "INV-000001".
pub fn format_invoice_number(sequence: i64) -> String {
    format!("INV-{sequence:06}")
}

/// The provisional identifier shown for a draft before it is issued.
/// Drafts never consume a permanent number (FR-08).
pub fn draft_identifier(draft_id: i64) -> String {
    format!("مسودة-{draft_id}")
}

/// Extract the numeric sequence from an issued invoice number, if it matches
/// the expected format. Used for validation and sorting.
pub fn parse_invoice_number(invoice_number: &str) -> Option<i64> {
    let rest = invoice_number.strip_prefix("INV-")?;
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_six_digit_numbers() {
        assert_eq!(format_invoice_number(1), "INV-000001");
        assert_eq!(format_invoice_number(42), "INV-000042");
        assert_eq!(format_invoice_number(123456), "INV-123456");
    }

    #[test]
    fn parses_back() {
        assert_eq!(parse_invoice_number("INV-000042"), Some(42));
        assert_eq!(parse_invoice_number("INV-42"), Some(42));
        assert_eq!(parse_invoice_number("42"), None);
        assert_eq!(parse_invoice_number("INV-abc"), None);
    }

    #[test]
    fn drafts_have_provisional_ids() {
        assert_eq!(draft_identifier(7), "مسودة-7");
    }
}
