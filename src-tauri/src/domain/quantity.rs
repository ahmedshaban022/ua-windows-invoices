//! Quantity handling.
//!
//! Quantities are stored as integer **thousandths** (`i64`) so that fractional
//! quantities such as 1.5 kg are exact. Line totals are rounded half-up in the
//! Rust domain layer (PRD §6 / §7), never in the frontend.

use crate::domain::error::{AppError, AppResult};
use crate::domain::money::normalize_digits;

pub const QUANTITY_SCALE: i64 = 1000;

/// Maximum absolute quantity, guarding against overflow in `i128` math.
const MAX_QUANTITY_THOUSANDTHS: i64 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity(i64);

impl Quantity {
    /// Build a quantity from thousandths.
    pub fn from_thousandths(v: i64) -> AppResult<Self> {
        if v <= 0 {
            return Err(AppError::validation("الكمية يجب أن تكون أكبر من صفر"));
        }
        if v > MAX_QUANTITY_THOUSANDTHS {
            return Err(AppError::validation("الكمية أكبر من الحد المسموح"));
        }
        Ok(Self(v))
    }

    /// Parse a user-typed quantity, e.g. "1.5" -> 1500, "٢٫٥" -> 2500.
    /// Up to 3 decimal places are accepted; extra digits are rounded half-up.
    pub fn parse(text: &str) -> AppResult<Self> {
        let cleaned = normalize_digits(text);
        let cleaned = cleaned.replace('٫', ".").replace(',', "");
        let cleaned = cleaned.trim();
        if cleaned.is_empty() {
            return Err(AppError::validation("الكمية مطلوبة"));
        }
        let (int_part, frac_part) = match cleaned.split_once('.') {
            Some((a, b)) => (a, b),
            None => (cleaned, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(AppError::validation("الكمية غير صحيحة"));
        }
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
        {
            return Err(AppError::validation("الكمية غير صحيحة"));
        }
        let int_value: i64 = if int_part.is_empty() {
            0
        } else {
            int_part
                .parse()
                .map_err(|_| AppError::validation("الكمية غير صحيحة"))?
        };
        let mut thousandths = int_value
            .checked_mul(QUANTITY_SCALE)
            .ok_or_else(|| AppError::validation("الكمية أكبر من الحد المسموح"))?;
        let mut frac_value: i64 = 0;
        let mut digits: u32 = 0;
        let mut round_up = false;
        for (index, ch) in frac_part.chars().enumerate() {
            if index >= 3 {
                // Round half-up on the first dropped digit.
                let digit = ch.to_digit(10).unwrap_or(0);
                round_up = digit >= 5;
                break;
            }
            let digit = ch.to_digit(10).unwrap_or(0) as i64;
            frac_value = frac_value * 10 + digit;
            digits += 1;
        }
        // The fraction is scaled to exactly three decimal places.
        frac_value = frac_value
            .checked_mul(
                10i64
                    .checked_pow(3 - digits)
                    .ok_or_else(|| AppError::validation("الكمية غير صحيحة"))?,
            )
            .ok_or_else(|| AppError::validation("الكمية أكبر من الحد المسموح"))?;
        thousandths = thousandths
            .checked_add(frac_value)
            .ok_or_else(|| AppError::validation("الكمية أكبر من الحد المسموح"))?;
        if round_up {
            thousandths = thousandths
                .checked_add(1)
                .ok_or_else(|| AppError::validation("الكمية أكبر من الحد المسموح"))?;
        }
        Self::from_thousandths(thousandths)
    }

    pub fn thousandths(self) -> i64 {
        self.0
    }

    /// Render as a plain decimal string, e.g. 1500 -> "1.5", 2000 -> "2".
    pub fn to_decimal_string(self) -> String {
        let major = self.0 / QUANTITY_SCALE;
        let frac = self.0 % QUANTITY_SCALE;
        if frac == 0 {
            format!("{major}")
        } else if frac % 100 == 0 {
            format!("{major}.{}", frac / 100)
        } else if frac % 10 == 0 {
            format!("{major}.{:02}", frac / 10)
        } else {
            format!("{major}.{frac:03}")
        }
    }
}

/// Compute a line total in minor units: round_half_up(quantity * unit_price).
/// The intermediate is `i128` so large quantities cannot overflow.
pub fn line_total_minor(quantity: Quantity, unit_price_minor: i64) -> i64 {
    let numerator = (quantity.thousandths() as i128) * (unit_price_minor as i128);
    let denominator = QUANTITY_SCALE as i128;
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    // Half-up on the fractional part (remainder is non-negative for non-negative inputs).
    let rounded = if remainder * 2 >= denominator {
        quotient + 1
    } else {
        quotient
    };
    // Saturate instead of wrapping: an absurd quantity must never produce a
    // negative total.
    rounded.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quantities() {
        assert_eq!(Quantity::parse("1").unwrap().thousandths(), 1000);
        assert_eq!(Quantity::parse("1.5").unwrap().thousandths(), 1500);
        assert_eq!(Quantity::parse("0.001").unwrap().thousandths(), 1);
        assert_eq!(Quantity::parse("٢٫٥").unwrap().thousandths(), 2500);
        assert_eq!(Quantity::parse("2.0005").unwrap().thousandths(), 2001);
    }

    #[test]
    fn rejects_non_positive_and_malformed() {
        assert!(Quantity::parse("0").is_err());
        assert!(Quantity::parse("-1").is_err());
        assert!(Quantity::parse("abc").is_err());
        assert!(Quantity::parse("").is_err());
    }

    #[test]
    fn renders_decimal_strings() {
        assert_eq!(
            Quantity::from_thousandths(1500)
                .unwrap()
                .to_decimal_string(),
            "1.5"
        );
        assert_eq!(
            Quantity::from_thousandths(2000)
                .unwrap()
                .to_decimal_string(),
            "2"
        );
        assert_eq!(
            Quantity::from_thousandths(1050)
                .unwrap()
                .to_decimal_string(),
            "1.05"
        );
        assert_eq!(
            Quantity::from_thousandths(1).unwrap().to_decimal_string(),
            "0.001"
        );
    }

    #[test]
    fn line_total_rounds_half_up() {
        let q = Quantity::from_thousandths(1500).unwrap();
        // 1.5 * 12.50 = 18.75 -> 1875
        assert_eq!(line_total_minor(q, 1250), 1875);

        // 0.5 * 0.05 = 0.025 -> rounds up to 0.03 -> 3
        let half = Quantity::from_thousandths(500).unwrap();
        assert_eq!(line_total_minor(half, 5), 3);

        // 3 * 10 = 30
        let three = Quantity::from_thousandths(3000).unwrap();
        assert_eq!(line_total_minor(three, 1000), 3000);
    }

    #[test]
    fn line_total_does_not_overflow() {
        let q = Quantity::from_thousandths(1_000_000_000_000).unwrap();
        let total = line_total_minor(q, i64::MAX / 2);
        assert!(total > 0);
    }
}
