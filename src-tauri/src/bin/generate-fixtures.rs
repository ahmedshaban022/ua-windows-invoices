//! Generate the test fixtures used by the automated checks and by the manual
//! verification pass.
//!
//! ```text
//! cargo run --manifest-path src-tauri/Cargo.toml --bin generate-fixtures
//! ```
//!
//! It writes into `tests/fixtures/`:
//!   * `catalog-10000.xlsx`   — 10,000 Arabic products for the scale check
//!   * `import-template.xlsx`  — the sample import template
//!   * `import-mixed.xlsx`     — a file with blank, duplicate and invalid rows

use std::path::PathBuf;

use rust_xlsxwriter::Workbook;

fn fixtures_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/src-tauri -> one level up is the repo root.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("tests");
    path.push("fixtures");
    path
}

/// Arabic product names, combined into 10,000 distinct products.
const ADJECTIVES: [&str; 20] = [
    "حليب",
    "جبن",
    "زبدة",
    "عسل",
    "أرز",
    "سكر",
    "ملح",
    "شاي",
    "قهوة",
    "زيت",
    "معكرونة",
    "عدس",
    "فول",
    "لحم",
    "دجاج",
    "سمك",
    "خضار",
    "فاكهة",
    "عصير",
    "ماء",
];

const NOUNS: [&str; 25] = [
    "كامل الدسم",
    "منقوش",
    "طري",
    "جاف",
    "عضوي",
    "محلي",
    "مستورد",
    "كبير",
    "صغير",
    "طازج",
    "معلب",
    "مجمّد",
    "صناعي",
    "طبيعي",
    "بلا سكر",
    "خالٍ من الدهن",
    "عبوة",
    "كرتون",
    "كيس",
    "برطمان",
    "زجاجة",
    "علبة",
    "صينية",
    "كوب",
    "ملعقة",
];

fn product_name(index: usize) -> String {
    let adjective = ADJECTIVES[index % ADJECTIVES.len()];
    let noun = NOUNS[(index / ADJECTIVES.len()) % NOUNS.len()];
    let batch = index / (ADJECTIVES.len() * NOUNS.len());
    if batch == 0 {
        format!("{adjective} {noun}")
    } else {
        format!("{adjective} {noun} — دفعة {batch}")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fixtures_dir();
    std::fs::create_dir_all(&dir)?;
    println!("writing fixtures into {}", dir.display());

    // ---------------------------------------------------------- catalog-10000
    let path = dir.join("catalog-10000.xlsx");
    {
        let mut workbook = Workbook::new();
        {
            let sheet = workbook.add_worksheet();
            let header = rust_xlsxwriter::Format::new().set_bold();
            let price = rust_xlsxwriter::Format::new().set_num_format("#,##0.00");
            sheet.write_row_with_format(0, 0, ["اسم المنتج", "السعر", "الرمز (SKU)"], &header)?;
            for index in 0..10_000 {
                let row = (index + 1) as u32;
                let price_value = 5.0 + ((index % 500) as f64) * 0.37;
                sheet.write_string(row, 0, product_name(index))?;
                sheet.write_number_with_format(row, 1, price_value, &price)?;
                sheet.write_string(row, 2, format!("SKU-{index:05}"))?;
            }
            sheet.set_column_width(0, 34)?;
            sheet.set_column_width(1, 14)?;
            sheet.set_column_width(2, 18)?;
        }
        workbook.save(&path)?;
    }
    println!("  catalog-10000.xlsx   (10,000 products)");

    // --------------------------------------------------------- import-template
    let path = dir.join("import-template.xlsx");
    {
        let mut workbook = Workbook::new();
        {
            let sheet = workbook.add_worksheet();
            let header = rust_xlsxwriter::Format::new().set_bold();
            let price = rust_xlsxwriter::Format::new().set_num_format("#,##0.00");
            sheet.write_row_with_format(0, 0, ["اسم المنتج", "السعر", "الرمز (SKU)"], &header)?;
            sheet.write_string(1, 0, "مثال منتج")?;
            sheet.write_number_with_format(2, 1, 25.0, &price)?;
            sheet.write_string(3, 2, "SKU-001")?;
            sheet.set_column_width(0, 34)?;
            sheet.set_column_width(1, 14)?;
            sheet.set_column_width(2, 18)?;
        }
        workbook.save(&path)?;
    }
    println!("  import-template.xlsx");

    // ----------------------------------------------------------- import-mixed
    let path = dir.join("import-mixed.xlsx");
    {
        let mut workbook = Workbook::new();
        {
            let sheet = workbook.add_worksheet();
            let header = rust_xlsxwriter::Format::new().set_bold();
            let price = rust_xlsxwriter::Format::new().set_num_format("#,##0.00");
            sheet.write_row_with_format(0, 0, ["اسم المنتج", "السعر", "الرمز (SKU)"], &header)?;
            // new, duplicate (twice), blank name, negative price, valid.
            let rows: Vec<(&str, &str, &str)> = vec![
                ("منتج جديد", "25.50", "NEW-1"),
                ("منتج مكرر", "10.00", "DUP-1"),
                ("منتج مكرر", "10.00", "DUP-1"),
                ("", "15.00", "BLANK-1"),
                ("سعر سالب", "-5.00", "NEG-1"),
                ("منتج ثانٍ", "99.99", "NEW-2"),
            ];
            for (index, (name, price_value, sku)) in rows.iter().enumerate() {
                let row = (index + 1) as u32;
                sheet.write_string(row, 0, *name)?;
                sheet.write_number_with_format(
                    row,
                    1,
                    price_value.parse::<f64>().unwrap_or(0.0),
                    &price,
                )?;
                sheet.write_string(row, 2, *sku)?;
            }
            sheet.set_column_width(0, 34)?;
            sheet.set_column_width(1, 14)?;
            sheet.set_column_width(2, 18)?;
        }
        workbook.save(&path)?;
    }
    println!("  import-mixed.xlsx    (blank / duplicate / invalid rows)");

    Ok(())
}
