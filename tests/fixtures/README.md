# Test fixtures

The fixtures are **generated** by a small Rust binary, and the generated files
are committed so the repository is self-contained and a fresh clone can run the
scale check and the import dry-run without running the generator first.

To regenerate them after changing the generator, run:

```powershell
npm run fixtures
# → cargo run --manifest-path src-tauri/Cargo.toml --bin generate-fixtures
```

| File | What it is | Used by |
| --- | --- | --- |
| `catalog-10000.xlsx` | 10,000 Arabic products with SKU and numeric prices | the scale check (PRD acceptance criterion 4) and manual performance testing |
| `import-template.xlsx` | the sample import template (name / price / SKU) | the import wizard; also produced by the app itself |
| `import-mixed.xlsx` | a workbook with a new row, a duplicate pair, a blank name and a negative price | the dry-run preview (PRD acceptance criterion 5) |

The generator lives in `src-tauri/src/bin/generate-fixtures.rs`.

---

## The 10,000-product scale check

`src-tauri/tests/catalog_scale.rs` seeds 10,000 products into an in-memory
database and asserts that:

* the catalog count is exactly 10,000,
* the list is **paged** (one page never returns more than the page size),
* a mid-word Arabic substring ("كامل" inside "حليب كامل الدسم") still matches
  through the FTS5 trigram index,
* the search finishes well inside the "fluid" budget,
* the combobox never returns more than ten suggestions.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test catalog_scale -- --nocapture
```

The test prints the seeding and search times so a regression is visible in the
CI log.
