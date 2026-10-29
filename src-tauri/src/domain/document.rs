//! The canonical invoice document model.
//!
//! Preview, PDF, print and image export all render **this** structure, so the
//! four outputs can never disagree about a number (PRD FR-07). The structure
//! carries only already-formatted strings: every amount was computed and
//! formatted by the Rust domain layer before it reached here.

use serde::{Deserialize, Serialize};

use crate::domain::invoice::SellerSnapshot;
use crate::domain::money::Currency;
use crate::domain::words::amount_in_words;
use crate::services::invoicing::{Invoice, InvoiceStatus};

/// One rendered invoice line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLine {
    pub name: String,
    /// Formatted quantity, e.g. "2.5".
    pub quantity_display: String,
    /// Formatted unit price, e.g. "12.50".
    pub unit_price_display: String,
    /// Formatted line total, e.g. "31.25".
    pub line_total_display: String,
}

/// The seller block shown in the document header.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSeller {
    pub company_name: String,
    pub company_phone: String,
    pub company_address: String,
    /// Inline SVG markup of the logo captured when the invoice was issued.
    pub logo_svg: String,
    pub brand_color: String,
    pub date_format: String,
}

/// The complete invoice document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceDocument {
    /// The permanent number, or a provisional identifier for a draft.
    pub invoice_number: String,
    /// True while the invoice has not been issued yet.
    pub is_draft: bool,
    /// ISO date (`YYYY-MM-DD`).
    pub invoice_date: String,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub notes: Option<String>,
    /// The custom terms (بنود) printed at the end of the document.
    #[serde(default)]
    pub terms: Vec<String>,
    pub currency_code: String,
    pub currency_name_ar: String,
    pub seller: DocumentSeller,
    pub lines: Vec<DocumentLine>,
    /// Formatted invoice total.
    pub total_display: String,
    /// The total written out in Arabic words.
    pub total_words: String,
}

impl InvoiceDocument {
    /// Build the document from a stored invoice and the currency in use.
    pub fn build(invoice: &Invoice, currency: &Currency) -> Self {
        let seller = &invoice.seller_snapshot;
        let lines = invoice
            .lines
            .iter()
            .map(|line| DocumentLine {
                name: line.name.clone(),
                quantity_display: line.quantity_display.clone(),
                unit_price_display: currency.format_minor(line.unit_price_minor),
                line_total_display: currency.format_minor(line.line_total_minor),
            })
            .collect();

        Self {
            invoice_number: invoice.display_number.clone(),
            is_draft: invoice.status == InvoiceStatus::Draft,
            invoice_date: invoice.invoice_date.clone(),
            customer_name: invoice.customer_name.clone(),
            customer_phone: invoice.customer_phone.clone(),
            notes: invoice.notes.clone(),
            terms: invoice.terms.clone(),
            currency_code: currency.code.clone(),
            currency_name_ar: currency.name_ar.clone(),
            seller: DocumentSeller {
                company_name: seller.company_name.clone(),
                company_phone: seller.company_phone.clone(),
                company_address: seller.company_address.clone(),
                logo_svg: seller.logo_svg.clone(),
                brand_color: seller.brand_color.clone(),
                date_format: seller.date_format.clone(),
            },
            lines,
            total_display: currency.format_minor(invoice.total_minor),
            total_words: amount_in_words(invoice.total_minor, currency),
        }
    }

    /// Build the document for a seller snapshot that has no invoice yet
    /// (used by the live preview of an unsaved invoice).
    pub fn build_preview(
        input: PreviewInput<'_>,
        currency: &Currency,
        seller: &SellerSnapshot,
    ) -> Self {
        let rendered = input
            .lines
            .iter()
            .map(|line| DocumentLine {
                name: line.name.clone(),
                quantity_display: line.quantity_display.clone(),
                unit_price_display: currency.format_minor(line.unit_price_minor),
                line_total_display: currency.format_minor(line.line_total_minor),
            })
            .collect();

        Self {
            invoice_number: input.invoice_number.to_string(),
            is_draft: true,
            invoice_date: input.invoice_date.to_string(),
            customer_name: input.customer_name.to_string(),
            customer_phone: input.customer_phone.map(|value| value.to_string()),
            notes: input.notes.map(|value| value.to_string()),
            terms: input
                .terms
                .iter()
                .map(|term| term.trim().to_string())
                .collect(),
            currency_code: currency.code.clone(),
            currency_name_ar: currency.name_ar.clone(),
            seller: DocumentSeller {
                company_name: seller.company_name.clone(),
                company_phone: seller.company_phone.clone(),
                company_address: seller.company_address.clone(),
                logo_svg: seller.logo_svg.clone(),
                brand_color: seller.brand_color.clone(),
                date_format: seller.date_format.clone(),
            },
            lines: rendered,
            total_display: currency.format_minor(input.total_minor),
            total_words: amount_in_words(input.total_minor, currency),
        }
    }
}

/// The parts of a document that are known before the invoice is saved.
pub struct PreviewInput<'a> {
    pub invoice_number: &'a str,
    pub invoice_date: &'a str,
    pub customer_name: &'a str,
    pub customer_phone: Option<&'a str>,
    pub notes: Option<&'a str>,
    /// The custom terms (بنود) chosen for this invoice.
    pub terms: &'a [String],
    pub lines: &'a [crate::domain::invoice::ComputedLine],
    pub total_minor: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::invoice::{compute_invoice, InvoiceLineInput};

    fn line(name: &str, qty: &str, price: &str) -> InvoiceLineInput {
        InvoiceLineInput {
            product_id: None,
            name: name.into(),
            quantity: qty.into(),
            unit_price: price.into(),
        }
    }

    #[test]
    fn builds_preview_document_with_formatted_totals() {
        let egp = Currency::egp();
        let computed = compute_invoice(&[line("حليب", "2", "25.00")], &egp).unwrap();
        let seller = SellerSnapshot {
            company_name: "شركة النور".into(),
            company_phone: "0100".into(),
            company_address: "القاهرة".into(),
            logo_svg: "<svg/>".into(),
            brand_color: "#0b68e6".into(),
            currency_code: "EGP".into(),
            currency_exponent: 2,
            date_format: "dd/MM/yyyy".into(),
        };

        let document = InvoiceDocument::build_preview(
            PreviewInput {
                invoice_number: "مسودة",
                invoice_date: "2026-10-29",
                customer_name: "عميل",
                customer_phone: None,
                notes: None,
                terms: &[],
                lines: &computed.lines,
                total_minor: computed.total_minor,
            },
            &egp,
            &seller,
        );

        assert_eq!(document.total_display, "50.00");
        assert_eq!(document.lines[0].quantity_display, "2");
        assert_eq!(document.lines[0].unit_price_display, "25.00");
        assert!(document.total_words.contains("خمسون"));
        assert!(document.is_draft);
        let _ = open_in_memory().unwrap();
    }
}
