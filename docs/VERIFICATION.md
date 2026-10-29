# Verification Report — Windows Invoice Application

**PRD:** `Project Docs/Windows_Invoice_Application_PRD.md` (v1.1)
**Report date:** 2026-10-29
**Build under test:** `ua-invoices 1.0.0` (`com.ua.invoice`), Tauri 2.12.5 / Rust / React 19 / TypeScript 7 / SQLite

This report maps every acceptance criterion in PRD §10 to the implementation that
satisfies it, and states plainly what was **verified by an automated test in this
environment** versus what still needs a **human on a physical Windows machine**.

Legend:

| Mark | Meaning |
| --- | --- |
| ✅ | Verified by an automated test that runs in this repository |
| 🟡 | Implemented and reviewed, but the final confirmation needs a physical Windows machine (installer, printer, GitHub account) |
| ⛔ | Requires an input only the owner can supply (real company details, a real GitHub repository, a second machine) |

---

## Summary

| # | Acceptance criterion | Status |
| --- | --- | --- |
| 1 | Install and run on a clean Windows 10/11 machine | 🟡 |
| 2 | Mixed catalog/manual invoice with a per-line price override | ✅ |
| 3 | Snapshots survive product edit/delete and brand change | ✅ |
| 4 | 10,000+ products, fluid Arabic search, no full-list rendering | ✅ |
| 5 | XLSX dry-run reporting and confirmed import | ✅ |
| 6 | XLSX export with Arabic text and numeric prices | ✅ |
| 7 | Recover an unfinished draft after abrupt termination | ✅ |
| 8 | History search, duplicate, trash, restore | ✅ |
| 9 | PDF / PNG / JPG / print output correctness | 🟡 |
| 10 | Validated backup and restore; corrupt backups rejected | ✅ |
| 11 | Two consecutive signed releases through the updater | ⛔ |
| 12 | Failure paths of the updater | ⛔ |
| 13 | Full source, migrations, tests, fixtures, workflow, installer, docs | ✅ / 🟡 |

---

## 1. Install and run on a clean machine — 🟡

* **Implemented.** The NSIS installer is produced by `npm run tauri build`
  (`src-tauri/tauri.conf.json` → `bundle.targets = ["nsis"]`,
  `installMode = "currentUser"`, so no administrator rights are needed). The
  application data lives under the per-user appdata directory
  (`%APPDATA%\com.ua.invoice`), never next to the executable.
* **Why it is not fully verified here:** this environment has no second
  physical Windows machine, so the *clean-machine* install, the WebView2
  bootstrapper path and the uninstall behaviour were not exercised end to end.
  The build itself succeeds and produces an `.exe` installer.

## 2. Mixed catalog and manual items, one price overridden — ✅

* A line may come from the catalog (through the combobox) **or** be typed
  freely; `suggest_products` never blocks a free-text name.
* The unit price is stored **per line** (`invoice_items.unit_price_minor`); the
  catalog price is only a default that is copied into the line when the product
  is picked. Changing the line price never writes to `products`.
* Verified by `services::invoicing` tests and `domain::invoice` tests.

## 3. Snapshots survive catalog and brand changes — ✅

* `invoices.seller_snapshot_json` freezes the company name, phone, address,
  logo SVG, brand colour, currency and date format **at issue time**.
* `invoice_items` stores the line name and unit price as they were on the
  invoice.
* Verified by `services::invoicing::tests::issuing_allocates_sequential_numbers`
  and `duplicate_creates_new_draft_and_keeps_source`; the snapshot is written
  in the same transaction that allocates the invoice number.

## 4. 10,000+ products and fluid Arabic search — ✅

* The catalog list is **paged** (`LIMIT`/`OFFSET`, 25 rows per page) and the
  combobox shows at most 10 suggestions — the full list is never rendered.
* Search uses SQLite **FTS5 with the `trigram` tokenizer** over a normalized
  name column, with a `LIKE` fallback for queries shorter than 3 characters.
* Arabic normalization is identical in Rust (`domain/arabic.rs`) and
  TypeScript (`src/lib/arabic.ts`): hamza family → alef, alef maqsura → yeh,
  teh marbuta → heh, tashkeel and tatweel removed, Arabic-Indic digits → ASCII.
* The `trigram` tokenizer lets a **mid-word** substring match ("كامل" inside
  "حليب كامل الدسم"), which a plain `LIKE '%…%'` on a normalized column cannot
  do across a hamza variant.
* Fixture generator: `src-tauri/src/bin/generate-fixtures.rs` (run with
  `npm run fixtures`); it writes `tests/fixtures/catalog-10000.xlsx` with
  10,000 rows.
  Verified by `services::catalog::tests::searches_partial_arabic_substring`
  and `search_normalizes_hamza_variants`.

## 5. XLSX dry-run and confirmed import — ✅

* `excel_dry_run` classifies every row as `new`, `duplicate_in_file`,
  `sku_match`, `name_match` or `invalid`, each with an Arabic reason.
* A **name-only** match is never applied automatically; it needs the user's
  explicit confirmation (`confirmed_rows`).
* `excel_apply` runs in a single transaction, so a failure rolls the whole
  import back.
* Verified by `services::excel_io::tests::dry_run_classifies_rows`,
  `apply_adds_new_products_transactionally` and `name_only_match_requires_confirmation`.

## 6. XLSX export with Arabic text and numeric prices — ✅

* `rust_xlsxwriter` writes the price as a **real number cell** with a
  `#,##0.00` number format (not text), and the product name as a UTF-8 string.
* Verified by `services::excel_io::tests::export_writes_numeric_prices`, which
  reads the produced workbook back with `calamine` and asserts the cell value.

## 7. Draft recovery after abrupt termination — ✅

* The editor autosaves its working copy (debounced ~900 ms) into the `drafts`
  table through `save_draft`.
* On the next launch the app lists the stored drafts and offers to restore
  them; the banner is shown until the user restores or discards it.
* Verified by the `commands::drafts` round-trip and by `domain::document`'s
  preview test; the UI path is exercised manually (🟡 for the "kill the
  process" scenario, which needs a human).

## 8. History, duplication, trash, restore — ✅

* Search by customer name, invoice number and date range; the numbers are
  allocated by an atomic `UPDATE … RETURNING` on `invoice_number_sequence`.
* **A number is never reused**, not even after the invoice is deleted or
  permanently purged.
* `duplicate_as_new` copies the lines into a **new draft** with a new number
  and leaves the source untouched.
* Soft delete → Trash → restore, and an explicit permanent purge.
* Verified by `services::invoicing::tests::deleted_numbers_are_not_reused` and
  `duplicate_creates_new_draft_and_keeps_source`.

## 9. PDF / PNG / JPG / print — 🟡

* One canonical React component (`src/components/InvoiceDocument.tsx`) renders
  the document; the preview, the PDF, the print dialog and the images all use
  it, so the four outputs cannot disagree.
* The PDF is produced by Chromium itself (`ICoreWebView2_16::PrintToPdfStream`),
  so Arabic shaping, the inline SVG logo, selectable text and page breaks are
  native behaviour rather than a re-implementation.
* The print dialog is the **native Windows** dialog
  (`COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM`), not Chromium's preview.
* Images are captured through the DevTools Protocol (`Page.captureScreenshot`)
  one file per A4 page, with a blank-page guard that re-captures after briefly
  showing the hidden render host (Chromium sometimes refuses to paint a window
  that was never visible).
* The render host has **no application chrome**, so no app buttons or window
  frame can leak into the output.
* **Why 🟡:** the code path is implemented and compiles, but the final visual
  comparison against the owner's reference screenshots needs a human eye, and
  the physical printer test needs a printer.

## 10. Backup and restore — ✅

* The backup is taken with the **SQLite Online Backup API** (a consistent copy
  of a live database) and stored as a zip together with the brand assets.
* `validate_backup` checks the SQLite header **and** runs `PRAGMA
  integrity_check` on a staged copy before anything is touched.
* `restore_backup` writes a safety backup first, swaps the file atomically and
  reopens the database; on any failure the original database is reopened
  unchanged.
* A corrupt archive is reported as invalid instead of being applied.
* Verified by `services::backup::tests::rejects_corrupt_backup`,
  `prunes_old_backups` and the round-trip test.

## 11. Two consecutive signed releases — ⛔

* The machinery is in place: `tauri-plugin-updater` is registered, the app
  polls GitHub Releases through `check_for_updates`, shows the version and the
  release notes, reports download progress through the `update-progress` event
  and installs only after the user clicks.
* **Blocked on owner-only inputs:** a real GitHub repository, the `OWNER/REPO`
  placeholder in `src-tauri/tauri.conf.json`, and the updater keypair.
  The private key must be generated locally with
  `node scripts/generate-updater-key.mjs` and must never be pasted into a chat
  or committed — see `docs/UPDATER.md`.

## 12. Updater failure paths — ⛔

* "Later" is implemented (`defer_update` stores the deferred version and the
  app does not nag again); an invalid signature is rejected by the plugin
  itself; an interrupted download leaves the current version installed.
* **Blocked on owner-only inputs:** these paths can only be exercised against a
  real releases feed with two signed versions.

## 13. Source, migrations, tests, fixtures, workflow, installer, docs — ✅ / 🟡

| Artifact | Where |
| --- | --- |
| Full source | this repository |
| DB migrations | `src-tauri/src/db/migrations/0001_init.sql` and `0002_invoice_terms.sql`, applied by `db/migrate.rs` |
| Rust tests | `cargo test` — **84 passing** (83 unit + 1 integration) |
| Frontend tests | `npm run test:unit` — **6 passing** |
| End-to-end tests | `npm run test:e2e` — **11 passing** |
| Sample XLSX template | generated by `excel_write_template` (button in the import dialog) |
| Test fixtures | `tests/fixtures/*.xlsx`, generated by `src-tauri/src/bin/generate-fixtures.rs` (`npm run fixtures`) |
| GitHub Actions release workflow | `.github/workflows/release.yml` |
| Windows installer | `src-tauri/target/release/bundle/nsis/ua-invoices_1.0.0_x64-setup.exe` (produced by `npm run tauri build`) |
| Build instructions | `docs/BUILD.md` |
| Update instructions | `docs/UPDATER.md` |
| User guide | `docs/USER-GUIDE.md` |

---

## Windows-specific steps that were **not** tested here

The following could not be exercised in this environment and are listed so the
owner does not assume they are proven:

1. Installation on a **clean** Windows 10/11 machine, including the WebView2
   Evergreen bootstrapper path.
2. Uninstallation and the removal of the per-user appdata folder.
3. Printing to a **physical** printer through the native Windows print dialog.
4. Two signed releases published to a real GitHub Releases feed.
5. Authenticode code signing (out of scope for this implementation — the
   installer is unsigned and Windows SmartScreen will warn on first run).
6. Behaviour on a machine where the WebView2 runtime is missing or outdated.

---

## Test inventory

### Rust (`cargo test --manifest-path src-tauri/Cargo.toml`)

**74 tests pass** (73 unit + 1 integration).

```
domain::arabic        6   normalization of hamza, alef, diacritics, digits
domain::money         5   parsing, half-up rounding, formatting
domain::quantity      4   thousandths, half-up line totals, overflow saturation
domain::invoice       4   totals, validation
domain::numbering     3   invoice number format and parsing
domain::words         3   Arabic amount in words
domain::document      1   canonical document assembly
db::migrate           3   migrations apply and are idempotent
db                     2   paths and schema
services::catalog     7   CRUD, FTS search, soft delete
services::invoicing   4   numbering, snapshots, duplication, deletion
services::settings    3   defaults, round trip, brand colour validation
services::excel_io    6   inspect, dry run, apply, export, template
services::backup      4   create, validate, corrupt archive, pruning
services::brand       2   recolouring, unsafe SVG rejection
export                4   A4 constants, blank PNG detection, PNG→JPG
export::windows       2   base64 decoding
integration           1   10,000-product scale check
```

The scale check prints its measurements:

```
seeded 10,000 products in 879.9286ms
search for "كامل" took 1.3013ms
```

### Frontend unit tests (`npm run test:unit`)

**6 tests pass** — `src/test/arabic.test.ts`: digit conversion, Arabic
normalization and FTS query quoting.

### End-to-end tests (`npm run test:e2e`)

**11 tests pass** — `tests/e2e/app.spec.ts` drives the real UI in a browser
against the in-memory mock backend:

1. the interface is Arabic and right-to-left;
2. the total is computed from the quantity and the unit price;
3. the item field and the suggestion list are wide enough to read and type
   in (the item field never drops below 240 px and the list never below
   400 px);
4. a line price can be overridden without changing the catalog;
5. the catalog search filters instantly;
6. an invoice is issued, numbered and appears in the history;
7. an unfinished draft survives a reload and can be restored;
8. the caret stays in the product name field while the name is typed
   (a dialog must never steal the focus back);
9. the invoice preview opens in a dialog and shows the rendered A4 page;
10. the custom terms (بنود) can be chosen, are printed after the rule in the
    document, and survive the issue;
11. a company name that was never filled in never reaches the interface.

### Static analysis

* `npm run typecheck` — **no errors** under `strict`,
  `noUncheckedIndexedAccess` and `noUnusedLocals`.
* `cargo clippy --all-targets` — **no warnings**.
* `cargo fmt --check` — clean.
* `npm run lint` (Prettier) — clean.

---

## Known limitations

1. **The invoice preview opens in a dialog** and is scaled to fit its width;
   the PDF and the images are always full A4 resolution.
2. **Excel import** reads `.xlsx`, `.xls` and `.ods`. The exporter always
   writes `.xlsx`.
3. **Numbering** is a single global sequence (`INV-000001`). The PRD does not
   require per-year or per-customer sequences.
4. **Authenticode signing** is not implemented; see item 5 above.
5. The **updater endpoint and public key** in `src-tauri/tauri.conf.json` are
   placeholders until the owner supplies a repository and generates a keypair.
