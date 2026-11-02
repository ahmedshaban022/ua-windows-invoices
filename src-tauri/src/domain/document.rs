//! The canonical invoice document model.
//!
//! Preview, PDF, print and image export all render **this** structure, so the
//! four outputs can never disagree about a number (PRD FR-07). The structure
//! carries only already-formatted strings: every amount was computed and
//! formatted by the Rust domain layer before it reached here.

use serde::{Deserialize, Serialize};

use crate::domain::invoice::{ComputedInvoice, SellerSnapshot};
use crate::domain::money::Currency;
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
    /// The sum of the line totals, before the discount.
    pub subtotal_display: String,
    /// The discount amount, formatted. `None` when the invoice has no
    /// discount.
    pub discount_display: Option<String>,
    /// The discount rate, e.g. "10%". `None` for a fixed-amount discount.
    pub discount_rate_display: Option<String>,
    /// Formatted invoice total: the subtotal minus the discount.
    pub total_display: String,
}

/// Render a percentage stored as hundredths of a percent, e.g. 1050 -> "10.5%".
fn format_percent(hundredths: i64) -> String {
    let major = hundredths / 100;
    let frac = hundredths % 100;
    if frac == 0 {
        format!("{major}%")
    } else if frac % 10 == 0 {
        format!("{major}.{}%", frac / 10)
    } else {
        format!("{major}.{frac:02}%")
    }
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

        let subtotal_minor: i64 = invoice.lines.iter().map(|line| line.line_total_minor).sum();
        let has_discount = invoice.discount_minor > 0;

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
            subtotal_display: currency.format_minor(subtotal_minor),
            discount_display: has_discount.then(|| currency.format_minor(invoice.discount_minor)),
            discount_rate_display: (has_discount && invoice.discount_kind == "percent")
                .then(|| format_percent(invoice.discount_value)),
            total_display: currency.format_minor(invoice.total_minor),
        }
    }

    /// Build the document for a seller snapshot that has no invoice yet
    /// (used by the live preview of an unsaved invoice).
    pub fn build_preview(
        input: PreviewInput<'_>,
        currency: &Currency,
        seller: &SellerSnapshot,
    ) -> Self {
        let computed = input.computed;
        let rendered = computed
            .lines
            .iter()
            .map(|line| DocumentLine {
                name: line.name.clone(),
                quantity_display: line.quantity_display.clone(),
                unit_price_display: currency.format_minor(line.unit_price_minor),
                line_total_display: currency.format_minor(line.line_total_minor),
            })
            .collect();

        let has_discount = computed.discount_minor > 0;

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
            subtotal_display: currency.format_minor(computed.subtotal_minor),
            discount_display: has_discount.then(|| currency.format_minor(computed.discount_minor)),
            discount_rate_display: (has_discount && computed.discount_kind == "percent")
                .then(|| format_percent(computed.discount_value)),
            total_display: currency.format_minor(computed.total_minor),
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
    /// The computed lines and totals, including the discount.
    pub computed: &'a ComputedInvoice,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::invoice::{compute_invoice, Discount, InvoiceLineInput};

    fn line(name: &str, qty: &str, price: &str) -> InvoiceLineInput {
        InvoiceLineInput {
            product_id: None,
            name: name.into(),
            quantity: qty.into(),
            unit_price: price.into(),
        }
    }

    fn seller() -> SellerSnapshot {
        SellerSnapshot {
            company_name: "شركة النور".into(),
            company_phone: "0100".into(),
            company_address: "القاهرة".into(),
            logo_svg: "<svg/>".into(),
            brand_color: "#0b68e6".into(),
            currency_code: "EGP".into(),
            currency_exponent: 2,
            date_format: "dd/MM/yyyy".into(),
        }
    }

    #[test]
    fn builds_preview_document_with_formatted_totals() {
        let egp = Currency::egp();
        let computed =
            compute_invoice(&[line("حليب", "2", "25.00")], &Discount::default(), &egp).unwrap();
        let seller = seller();

        let document = InvoiceDocument::build_preview(
            PreviewInput {
                invoice_number: "مسودة",
                invoice_date: "2026-10-29",
                customer_name: "عميل",
                customer_phone: None,
                notes: None,
                terms: &[],
                computed: &computed,
            },
            &egp,
            &seller,
        );

        assert_eq!(document.total_display, "50.00");
        assert_eq!(document.subtotal_display, "50.00");
        assert_eq!(document.discount_display, None);
        assert_eq!(document.discount_rate_display, None);
        assert_eq!(document.lines[0].quantity_display, "2");
        assert_eq!(document.lines[0].unit_price_display, "25.00");
        assert!(document.is_draft);
        let _ = open_in_memory().unwrap();
    }

    #[test]
    fn the_preview_document_shows_the_discount_breakdown() {
        let egp = Currency::egp();
        let discount = Discount::parse(
            Some(&crate::domain::invoice::DiscountInput {
                kind: "percent".into(),
                value: "10".into(),
            }),
            &egp,
        )
        .unwrap();
        let computed = compute_invoice(&[line("حليب", "2", "25.00")], &discount, &egp).unwrap();

        let document = InvoiceDocument::build_preview(
            PreviewInput {
                invoice_number: "مسودة",
                invoice_date: "2026-10-29",
                customer_name: "عميل",
                customer_phone: None,
                notes: None,
                terms: &[],
                computed: &computed,
            },
            &egp,
            &seller(),
        );

        assert_eq!(document.subtotal_display, "50.00");
        assert_eq!(document.discount_display.as_deref(), Some("5.00"));
        assert_eq!(document.discount_rate_display.as_deref(), Some("10%"));
        assert_eq!(document.total_display, "45.00");
    }
}
