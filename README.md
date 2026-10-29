# ua-invoices — Windows desktop invoicing (Arabic RTL)

A complete, offline **Windows desktop application** for creating and issuing
invoices in Arabic with a right-to-left interface.

* **Tauri 2** + **Rust** backend, **React 19** + **TypeScript** frontend
* **SQLite** (WAL, FTS5 trigram index) stored in the per-user appdata folder
* One canonical invoice document drives the **preview, the PDF, the print
  dialog and the image export**, so the four can never disagree
* No server, no account, no telemetry — invoice data never leaves the machine

Built from `Project Docs/Windows_Invoice_Application_PRD.md` (v1.1).

---

## Quick start

```powershell
npm ci
npm run icons        # generate the Windows icon set from the brand SVG
npm run tauri:dev    # run the app
```

Build the installer:

```powershell
npm run tauri:build
# → src-tauri/target/release/bundle/nsis/ua-invoices_1.0.0_x64-setup.exe
```

Full instructions: [`docs/BUILD.md`](docs/BUILD.md).

---

## What it does

| Area | Summary |
| --- | --- |
| **Invoices** | Draft → issue, permanent non-reusable numbers (`INV-000001`), seller/brand snapshots frozen at issue time, per-line price override, live totals computed in Rust. |
| **Catalog** | Instant Arabic search (FTS5 trigram + `LIKE` fallback for short queries), paged list, soft delete. |
| **Excel** | Two-phase import (dry-run classification → confirmed transactional apply) and export with real numeric price cells. |
| **History** | Search by customer/date/number, duplicate as a new invoice, trash and restore, permanent purge. |
| **Export** | Real PDF (Chromium), native Windows print dialog, PNG/JPG per A4 page. |
| **Backup** | SQLite Online Backup API into a zip, validated and integrity-checked before any restore. |
| **Drafts** | Debounced autosave with recovery after an abrupt termination. |
| **Updates** | User-initiated, signature-verified updates from GitHub Releases. |

---

## Repository layout

```
src/                      React frontend (Arabic RTL)
  components/             UI kit, app shell, combobox, the canonical document
  pages/                  The five screens
  lib/                    Types, API layer, formatting, Arabic normalization
  styles/                 Design tokens, global styles, invoice document styles
src-tauri/                Rust backend
  src/domain/             Money, quantities, invoices, Arabic, words
  src/services/           Catalog, invoicing, settings, brand, Excel, backup
  src/commands/           The typed command layer exposed to the frontend
  src/export/             WebView2 PDF / print / image pipeline
  src/db/migrations/      Embedded SQL migrations
tests/e2e/                Playwright end-to-end suite
tests/fixtures/           Generated fixtures (10k catalog, templates)
docs/                     Build, updater, user guide, verification report
scripts/                  Icon generation, updater key generation, MSVC env
```

---

## Design rules that the code enforces

* **No binary floating point for money.** Every amount is an `i64` count of
  minor units (piastres for EGP); quantities are integer thousandths. Line
  totals are rounded half-up in `i128`.
* **Arabic search is normalized identically in Rust and TypeScript**, so a
  query typed in the UI matches what the database indexed.
* **The Rust side owns the arithmetic.** The frontend never sends a total; it
  asks Rust to compute it.
* **One document, four outputs.** The preview, the PDF, the print dialog and
  the images all render `src/components/InvoiceDocument.tsx`.

---

## Tests

```powershell
npm run test:unit     # frontend unit tests
npm run test:rust     # Rust unit + integration tests
npm run test:e2e      # Playwright end-to-end (needs a browser install)
npm run typecheck     # TypeScript, strict
```

Current status: **73 Rust tests**, **6 frontend unit tests**, TypeScript clean.

The acceptance criteria of the PRD and what is *not* verified on a physical
machine are tracked honestly in [`docs/VERIFICATION.md`](docs/VERIFICATION.md).

---

## Owner-only steps

These cannot be automated and are documented rather than guessed:

1. **Real company details** — set them in *Settings* (the defaults are Arabic
   placeholders: `اسم الشركة` / `رقم الهاتف` / `العنوان`).
2. **The updater keypair** — generate it locally:
   `node scripts/generate-updater-key.mjs`. The private key must never be
   committed or pasted into a chat. See [`docs/UPDATER.md`](docs/UPDATER.md).
3. **The GitHub repository** — replace the `<OWNER>/<REPO>` placeholder in
   `src-tauri/tauri.conf.json`.
4. **A second Windows machine** — for the clean-install, uninstall and printer
   checks.

---

## Brand

The owner logo was recolored to **`#0b68e6`** with the artwork preserved
exactly (`assets/brand/logo.svg`, `assets/brand/mark.svg`). The colour is a
replaceable token in *Settings*.
