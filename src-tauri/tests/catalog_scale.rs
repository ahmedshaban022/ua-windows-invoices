//! Scale check for the product catalog (PRD FR-02, acceptance criterion 4).
//!
//! Seeds 10,000 products into an in-memory database and verifies that the
//! Arabic search stays fast and correct, and that the *list* is paged rather
//! than materialized.

use std::time::Instant;

use ua_windows_invoices_lib::db::open_in_memory;
use ua_windows_invoices_lib::services::catalog::{self, ProductInput};

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

#[test]
fn ten_thousand_products_remain_searchable_and_paged() {
    let conn = open_in_memory().expect("in-memory database");

    let started = Instant::now();
    for index in 0..10_000usize {
        catalog::create(
            &conn,
            &ProductInput {
                name: product_name(index),
                base_price_minor: 100 + (index % 5_000) as i64,
                sku: Some(format!("SKU-{index:05}")),
            },
        )
        .expect("insert");
    }
    let seed_time = started.elapsed();
    println!("seeded 10,000 products in {seed_time:?}");

    assert_eq!(catalog::count(&conn).expect("count"), 10_000);

    // The list is paged: one page never returns the whole catalog.
    let page = catalog::search(&conn, "", 1, 25, false).expect("search");
    assert_eq!(page.items.len(), 25);
    assert_eq!(page.total, 10_000);

    // A mid-word Arabic substring still finds the product.
    let started = Instant::now();
    let found = catalog::search(&conn, "كامل", 1, 25, false).expect("search");
    let search_time = started.elapsed();
    println!("search for \"كامل\" took {search_time:?}");

    assert!(found.total >= 1, "expected at least one match");
    assert!(found
        .items
        .iter()
        .all(|product| product.name.contains("كامل")));

    // The search must stay well below the "fluid" budget.
    assert!(
        search_time.as_millis() < 500,
        "search took {search_time:?}, which is too slow"
    );

    // The combobox returns at most ten suggestions.
    let suggestions = catalog::suggest(&conn, "لبن", 10).expect("suggest");
    assert!(suggestions.len() <= 10);
}
