//! Invoice domain model: lines, totals, validation and seller snapshots.

use serde::{Deserialize, Serialize};

use crate::domain::error::{AppError, AppResult};
use crate::domain::money::Currency;
use crate::domain::quantity::{line_total_minor, Quantity};

/// A single invoice line as supplied by the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLineInput {
    /// Catalog product id, when the line came from the catalog.
    #[serde(default)]
    pub product_id: Option<i64>,
    /// The item name shown on the invoice.
    pub name: String,
    /// Quantity typed by the user (decimal text). Parsed by the Rust layer.
    pub quantity: String,
    /// Unit price typed by the user (decimal text). Parsed by the Rust layer.
    pub unit_price: String,
}

impl InvoiceLineInput {
    pub fn validate(&self) -> AppResult<()> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(AppError::validation("اسم البند مطلوب"));
        }
        if name.chars().count() > 500 {
            return Err(AppError::validation("اسم البند طويل جدًا"));
        }
        let _ = Quantity::parse(&self.quantity)?;
        Ok(())
    }
}

/// A fully computed line, ready to persist and render.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedLine {
    #[serde(default)]
    pub product_id: Option<i64>,
    pub name: String,
    pub quantity_thousandths: i64,
    pub quantity_display: String,
    pub unit_price_minor: i64,
    pub line_total_minor: i64,
    pub sort_order: i32,
}

/// The complete computed invoice, as returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedInvoice {
    pub lines: Vec<ComputedLine>,
    pub total_minor: i64,
    pub currency_code: String,
    pub currency_exponent: u32,
}

impl ComputedInvoice {
    pub fn total_display(&self, currency: &Currency) -> String {
        currency.format_minor(self.total_minor)
    }
}

/// Seller identity captured at issue time, so later settings changes never
/// rewrite historical invoices (FR-01, FR-06, FR-10).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SellerSnapshot {
    pub company_name: String,
    pub company_phone: String,
    pub company_address: String,
    /// Inline SVG markup of the logo in use at issue time.
    pub logo_svg: String,
    pub brand_color: String,
    pub currency_code: String,
    pub currency_exponent: u32,
    pub date_format: String,
}

impl Default for SellerSnapshot {
    fn default() -> Self {
        Self {
            company_name: String::new(),
            company_phone: String::new(),
            company_address: String::new(),
            logo_svg: String::new(),
            brand_color: "#0b68e6".into(),
            currency_code: "EGP".into(),
            currency_exponent: 2,
            date_format: "dd/MM/yyyy".into(),
        }
    }
}

/// Validate and compute the authoritative totals for a set of invoice lines.
///
/// The frontend never sends totals; Rust recomputes them (FR-01).
pub fn compute_invoice(
    lines: &[InvoiceLineInput],
    currency: &Currency,
) -> AppResult<ComputedInvoice> {
    if lines.is_empty() {
        return Err(AppError::validation("يجب إضافة بند واحد على الأقل"));
    }

    let mut computed = Vec::with_capacity(lines.len());
    let mut total: i64 = 0;

    for (index, line) in lines.iter().enumerate() {
        line.validate()?;
        let quantity = Quantity::parse(&line.quantity)?;
        let unit_price_minor = crate::domain::money::parse_price(&line.unit_price, currency)
            .ok_or_else(|| AppError::validation("سعر البند غير صحيح"))?;
        if unit_price_minor < 0 {
            return Err(AppError::validation("سعر البند لا يمكن أن يكون سالبًا"));
        }
        let line_total = line_total_minor(quantity, unit_price_minor);
        total = total
            .checked_add(line_total)
            .ok_or_else(|| AppError::validation("إجمالي الفاتورة كبير جدًا"))?;

        computed.push(ComputedLine {
            product_id: line.product_id,
            name: line.name.trim().to_string(),
            quantity_thousandths: quantity.thousandths(),
            quantity_display: quantity.to_decimal_string(),
            unit_price_minor,
            line_total_minor: line_total,
            sort_order: index as i32,
        });
    }

    Ok(ComputedInvoice {
        lines: computed,
        total_minor: total,
        currency_code: currency.code.clone(),
        currency_exponent: currency.exponent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(name: &str, qty: &str, price: &str) -> InvoiceLineInput {
        InvoiceLineInput {
            product_id: None,
            name: name.into(),
            quantity: qty.into(),
            unit_price: price.into(),
        }
    }

    #[test]
    fn computes_totals_from_lines() {
        let egp = Currency::egp();
        let result = compute_invoice(
            &[line("حليب", "2", "25.00"), line("خبز", "1", "10.50")],
            &egp,
        )
        .unwrap();
        assert_eq!(result.total_minor, 6050);
        assert_eq!(result.lines[0].line_total_minor, 5000);
        assert_eq!(result.lines[1].line_total_minor, 1050);
        assert_eq!(result.lines[0].sort_order, 0);
        assert_eq!(result.lines[1].sort_order, 1);
    }

    #[test]
    fn rejects_empty_invoice() {
        let egp = Currency::egp();
        assert!(compute_invoice(&[], &egp).is_err());
    }

    #[test]
    fn rejects_blank_name() {
        let egp = Currency::egp();
        assert!(compute_invoice(&[line("   ", "1", "5")], &egp).is_err());
    }

    #[test]
    fn rejects_zero_quantity() {
        let egp = Currency::egp();
        assert!(compute_invoice(&[line("حليب", "0", "5")], &egp).is_err());
    }

    #[test]
    fn rejects_negative_price() {
        let egp = Currency::egp();
        assert!(compute_invoice(&[line("حليب", "1", "-5")], &egp).is_err());
    }
}
