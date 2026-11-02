//! Invoice domain model: lines, totals, validation and seller snapshots.

use serde::{Deserialize, Serialize};

use crate::domain::error::{AppError, AppResult};
use crate::domain::money::{normalize_digits, Currency};
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
    /// The sum of the line totals, before the discount.
    pub subtotal_minor: i64,
    /// The discount that was applied, in minor units.
    pub discount_minor: i64,
    /// How the discount was expressed: "amount" or "percent".
    pub discount_kind: String,
    /// The value the user typed: minor units for an amount, hundredths of a
    /// percent for a percentage.
    pub discount_value: i64,
    /// The total due: the subtotal minus the discount.
    pub total_minor: i64,
    pub currency_code: String,
    pub currency_exponent: u32,
}

impl ComputedInvoice {
    pub fn total_display(&self, currency: &Currency) -> String {
        currency.format_minor(self.total_minor)
    }
}

/// How the user expressed the discount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscountKind {
    /// A fixed amount of money.
    Amount,
    /// A percentage of the subtotal.
    Percent,
}

impl Default for DiscountKind {
    fn default() -> Self {
        DiscountKind::Amount
    }
}

impl DiscountKind {
    /// Parse the kind sent by the frontend. Anything unknown means an amount.
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "percent" | "percentage" => DiscountKind::Percent,
            _ => DiscountKind::Amount,
        }
    }

    /// The value stored in the database and sent to the frontend.
    pub fn as_str(self) -> &'static str {
        match self {
            DiscountKind::Amount => "amount",
            DiscountKind::Percent => "percent",
        }
    }
}

/// The discount as it arrives from the frontend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountInput {
    /// "amount" or "percent".
    #[serde(default)]
    pub kind: String,
    /// The value the user typed, as decimal text.
    #[serde(default)]
    pub value: String,
}

/// A validated discount.
#[derive(Debug, Clone, Copy, Default)]
pub struct Discount {
    pub kind: DiscountKind,
    /// The value the user typed: minor units for an amount, hundredths of a
    /// percent for a percentage.
    pub value: i64,
}

impl Discount {
    /// Validate and convert the discount the frontend sent.
    ///
    /// An empty value means "no discount", which is what an invoice without a
    /// discount carries.
    pub fn parse(input: Option<&DiscountInput>, currency: &Currency) -> AppResult<Self> {
        let Some(input) = input else {
            return Ok(Self::default());
        };
        let kind = DiscountKind::parse(&input.kind);
        let text = input.value.trim();
        if text.is_empty() {
            return Ok(Self { kind, value: 0 });
        }
        let value = match kind {
            DiscountKind::Amount => currency
                .parse_major_to_minor(&normalize_digits(text))
                .ok_or_else(|| AppError::validation("قيمة الخصم غير صحيحة"))?,
            DiscountKind::Percent => parse_percent(text)?,
        };
        Ok(Self { kind, value })
    }
}

/// Parse a percentage typed by the user into hundredths of a percent.
///
/// "10" -> 1000, "10.5" -> 1050. Extra decimals are dropped.
fn parse_percent(text: &str) -> AppResult<i64> {
    let cleaned = normalize_digits(text).replace('٫', ".").replace(',', "");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return Ok(0);
    }
    let (int_part, frac_part) = match cleaned.split_once('.') {
        Some((a, b)) => (a, b),
        None => (cleaned, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(AppError::validation("نسبة الخصم غير صحيحة"));
    }
    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return Err(AppError::validation("نسبة الخصم غير صحيحة"));
    }
    let int_value: i64 = int_part
        .parse()
        .map_err(|_| AppError::validation("نسبة الخصم غير صحيحة"))?;
    let mut fraction: i64 = 0;
    let mut digits = 0u32;
    for ch in frac_part.chars().take(2) {
        fraction = fraction * 10 + ch.to_digit(10).unwrap_or(0) as i64;
        digits += 1;
    }
    fraction *= 10i64.pow(2 - digits);
    int_value
        .checked_mul(100)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| AppError::validation("نسبة الخصم غير صحيحة"))
}

/// The result of applying a discount to a subtotal.
#[derive(Debug, Clone, Copy)]
pub struct AppliedDiscount {
    /// The subtotal the discount was taken from.
    pub subtotal_minor: i64,
    /// The discount amount, in minor units.
    pub amount_minor: i64,
    /// The total due after the discount.
    pub total_minor: i64,
}

/// Apply a discount to a subtotal.
///
/// The discount never exceeds the subtotal, so the total can never go
/// negative and an invoice can never be worth less than nothing.
pub fn apply_discount(discount: &Discount, subtotal_minor: i64) -> AppResult<AppliedDiscount> {
    let amount_minor = match discount.kind {
        DiscountKind::Amount => discount.value.min(subtotal_minor).max(0),
        DiscountKind::Percent => {
            if discount.value > 10_000 {
                return Err(AppError::validation("نسبة الخصم لا يمكن أن تتجاوز 100%"));
            }
            // Half-up on subtotal * basis_points / 10000.
            let numerator = (subtotal_minor as i128) * (discount.value as i128);
            let rounded = (numerator + 5_000) / 10_000;
            (rounded as i64).min(subtotal_minor).max(0)
        }
    };
    Ok(AppliedDiscount {
        subtotal_minor,
        amount_minor,
        total_minor: subtotal_minor - amount_minor,
    })
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
    discount: &Discount,
    currency: &Currency,
) -> AppResult<ComputedInvoice> {
    if lines.is_empty() {
        return Err(AppError::validation("يجب إضافة بند واحد على الأقل"));
    }

    let mut computed = Vec::with_capacity(lines.len());
    let mut subtotal: i64 = 0;

    for (index, line) in lines.iter().enumerate() {
        line.validate()?;
        let quantity = Quantity::parse(&line.quantity)?;
        let unit_price_minor = crate::domain::money::parse_price(&line.unit_price, currency)
            .ok_or_else(|| AppError::validation("سعر البند غير صحيح"))?;
        if unit_price_minor < 0 {
            return Err(AppError::validation("سعر البند لا يمكن أن يكون سالبًا"));
        }
        let line_total = line_total_minor(quantity, unit_price_minor);
        subtotal = subtotal
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

    let applied = apply_discount(discount, subtotal)?;

    Ok(ComputedInvoice {
        lines: computed,
        subtotal_minor: applied.subtotal_minor,
        discount_minor: applied.amount_minor,
        discount_kind: discount.kind.as_str().to_string(),
        discount_value: discount.value,
        total_minor: applied.total_minor,
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
            &Discount::default(),
            &egp,
        )
        .unwrap();
        assert_eq!(result.total_minor, 6050);
        assert_eq!(result.subtotal_minor, 6050);
        assert_eq!(result.discount_minor, 0);
        assert_eq!(result.lines[0].line_total_minor, 5000);
        assert_eq!(result.lines[1].line_total_minor, 1050);
        assert_eq!(result.lines[0].sort_order, 0);
        assert_eq!(result.lines[1].sort_order, 1);
    }

    #[test]
    fn rejects_empty_invoice() {
        let egp = Currency::egp();
        assert!(compute_invoice(&[], &Discount::default(), &egp).is_err());
    }

    #[test]
    fn rejects_blank_name() {
        let egp = Currency::egp();
        assert!(compute_invoice(
            &[line("   ", "1", "5")],
            &Discount::default(),
            &egp
        )
        .is_err());
    }

    #[test]
    fn rejects_zero_quantity() {
        let egp = Currency::egp();
        assert!(compute_invoice(
            &[line("حليب", "0", "5")],
            &Discount::default(),
            &egp
        )
        .is_err());
    }

    #[test]
    fn rejects_negative_price() {
        let egp = Currency::egp();
        assert!(compute_invoice(
            &[line("حليب", "1", "-5")],
            &Discount::default(),
            &egp
        )
        .is_err());
    }

    #[test]
    fn applies_a_fixed_amount_discount() {
        let egp = Currency::egp();
        let discount = Discount::parse(
            Some(&DiscountInput {
                kind: "amount".into(),
                value: "250.00".into(),
            }),
            &egp,
        )
        .unwrap();
        assert_eq!(discount.value, 25000);

        let result =
            compute_invoice(&[line("حليب", "2", "25.00")], &discount, &egp).unwrap();
        assert_eq!(result.subtotal_minor, 5000);
        assert_eq!(result.discount_minor, 25000.min(5000));
        assert_eq!(result.total_minor, 0);
        assert_eq!(result.discount_kind, "amount");
    }

    #[test]
    fn applies_a_percentage_discount() {
        let egp = Currency::egp();
        let discount = Discount::parse(
            Some(&DiscountInput {
                kind: "percent".into(),
                value: "10".into(),
            }),
            &egp,
        )
        .unwrap();
        assert_eq!(discount.value, 1000);

        let result =
            compute_invoice(&[line("حليب", "2", "25.00")], &discount, &egp).unwrap();
        assert_eq!(result.subtotal_minor, 5000);
        assert_eq!(result.discount_minor, 500);
        assert_eq!(result.total_minor, 4500);
        assert_eq!(result.discount_kind, "percent");
    }

    #[test]
    fn a_percentage_discount_rounds_half_up() {
        let egp = Currency::egp();
        // 10% of 5.05 is 0.505 -> 0.51
        let discount = Discount::parse(
            Some(&DiscountInput {
                kind: "percent".into(),
                value: "10".into(),
            }),
            &egp,
        )
        .unwrap();
        let result = compute_invoice(&[line("حليب", "1", "5.05")], &discount, &egp).unwrap();
        assert_eq!(result.discount_minor, 51);
        assert_eq!(result.total_minor, 454);
    }

    #[test]
    fn a_discount_never_exceeds_the_subtotal() {
        let egp = Currency::egp();
        let discount = Discount::parse(
            Some(&DiscountInput {
                kind: "amount".into(),
                value: "999.00".into(),
            }),
            &egp,
        )
        .unwrap();
        let result =
            compute_invoice(&[line("حليب", "1", "25.00")], &discount, &egp).unwrap();
        assert_eq!(result.discount_minor, 2500);
        assert_eq!(result.total_minor, 0);
    }

    #[test]
    fn rejects_a_percentage_above_one_hundred() {
        let egp = Currency::egp();
        let discount = Discount::parse(
            Some(&DiscountInput {
                kind: "percent".into(),
                value: "120".into(),
            }),
            &egp,
        )
        .unwrap();
        assert!(compute_invoice(&[line("حليب", "1", "25.00")], &discount, &egp).is_err());
    }

    #[test]
    fn an_empty_discount_means_no_discount() {
        let egp = Currency::egp();
        let discount = Discount::parse(
            Some(&DiscountInput {
                kind: "percent".into(),
                value: "  ".into(),
            }),
            &egp,
        )
        .unwrap();
        assert_eq!(discount.value, 0);
        let result =
            compute_invoice(&[line("حليب", "1", "25.00")], &discount, &egp).unwrap();
        assert_eq!(result.discount_minor, 0);
        assert_eq!(result.total_minor, 2500);
    }

    #[test]
    fn reads_percentages_with_arabic_digits() {
        let egp = Currency::egp();
        let discount = Discount::parse(
            Some(&DiscountInput {
                kind: "percent".into(),
                value: "١٠٫٥".into(),
            }),
            &egp,
        )
        .unwrap();
        assert_eq!(discount.value, 1050);
    }

    #[test]
    fn rejects_a_malformed_discount() {
        let egp = Currency::egp();
        assert!(
            Discount::parse(
                Some(&DiscountInput {
                    kind: "amount".into(),
                    value: "abc".into(),
                }),
                &egp,
            )
            .is_err()
        );
        assert!(
            Discount::parse(
                Some(&DiscountInput {
                    kind: "percent".into(),
                    value: "-5".into(),
                }),
                &egp,
            )
            .is_err()
        );
    }
}
