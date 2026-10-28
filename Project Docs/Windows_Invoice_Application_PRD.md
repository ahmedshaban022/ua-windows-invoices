# Product Requirements Document (PRD)
## Windows Desktop Invoice Application

**Version:** 1.1 — Consolidated requirements, including in-app updates  
**Date:** 28 October 2026  
**Document language:** English  
**Application UI language:** Arabic (right-to-left)  
**Target platform:** Windows 10/11 desktop  
**Status:** Implementation-ready scope; brand assets and release repository still to be provided

---

## 1. Product Summary

Build an installable, fast, lightweight, **offline-first Windows desktop application** for creating, saving, previewing, finding, printing, and exporting invoices. The user enters the customer's details manually for each invoice, adds line items either from a searchable local product catalog or directly by typing them, and can retrieve every saved invoice later.

The product catalog supports manual CRUD and bulk Excel import/export. App data is stored locally in SQLite. The application can check for **signed software updates published through GitHub Releases**, inform the user, and install an update from within the app on request. Invoice creation, storage, and export must work without an internet connection; only update checking/downloading requires internet.

**This is an invoice creation and archival tool, NOT an accounting, payment-tracking, inventory, or CRM system.**

### Primary outcomes

1. Create accurate invoices with as few interactions as practical.
2. Find products quickly even when the catalog contains 10,000+ items.
3. Preserve previously issued invoices even if products, company settings, or logo change.
4. Safely keep local data and recover it using backups and autosaved drafts.
5. Produce professional Arabic-RTL invoice output as PDF, print, PNG, and JPG.
6. Allow the developer to ship signed releases that end users can install from an in-app update prompt, without manually downloading and reinstalling each version.

---

## 2. Scope

### Included in v1

- Windows installable desktop app, local SQLite database, Arabic RTL interface.
- Create invoice, preview invoice, save/issue invoice, autosave drafts, list and search previous invoices, duplicate an invoice, safe delete/restore.
- Manual customer name and phone per invoice; no separate customer directory.
- Product catalog with add/edit/delete, fast indexed search, import/export `.xlsx`.
- Select product in invoice or enter a completely manual item; optional explicit save of a manual item to catalog.
- Optional price override **for the current invoice line only**.
- Invoice PDF export, Windows print dialog, PNG/JPG image export, and practical file-sharing/open-folder options.
- Company identity configuration, original SVG logo, exact brand color, invoice snapshot of seller identity.
- Automatic and manual backups plus validated restore.
- Signed in-app updates via GitHub Releases, with an optional user-triggered download/install, progress indication, restart flow, and no invoice-data loss.
- Packaging, automated tests, and Windows installation/release documentation.

### Explicit exclusions

- Customer management, customer lookup, CRM, or saved customer profiles.
- Payment status, paid/unpaid, debts, receipts, installments, financial ledger, checkout, POS.
- Stock/inventory tracking, purchasing, supplier management, analytics dashboards.
- Multi-user login, roles/permissions, cloud synchronization, server-dependent invoice creation.
- Official tax authority or e-invoicing compliance integrations unless separately specified.
- Mandatory auto-installation without user consent.

---

## 3. Users and Key Workflows

**Primary user:** A small-business operator who issues invoices manually and needs to reuse a large catalog of products. Customers rarely repeat, so customer contact information must be entered afresh for every invoice.

### Journey A — Fast catalog-based invoice

1. Open **New Invoice**.
2. Type customer name and, if applicable, phone number.
3. Start typing a product name in an invoice line.
4. See up to 8–10 relevant search suggestions (name and base price), select by mouse or keyboard.
5. Enter quantity; unit price populates automatically; line and invoice totals calculate immediately.
6. Repeat for other items, optionally override an individual line price.
7. Save the invoice, view the final preview, then print or save PDF/image.

### Journey B — One-off product

1. Click **Add Manual Item** from the invoice editor.
2. Enter item name, unit price, and quantity.
3. Optionally explicitly select **Save item to product catalog** (off by default).
4. Finish the invoice without being forced to create a catalog record.

### Journey C — Bulk product onboarding

1. Open **Products** > **Import Excel**.
2. Select `.xlsx`, worksheet, and column mapping.
3. Preview parsed rows, invalid prices, missing names, duplicate/matched rows.
4. Choose skip/update/new-item behavior, review summary, confirm import.
5. Search imported products in an invoice immediately.

### Journey D — Find and reuse an old invoice

1. Open **Previous Invoices**.
2. Search by customer name, invoice number, or date range.
3. Preview, print, or export the existing invoice without changing historical data.
4. Optionally **Duplicate as New** to create a new draft from it.
5. Optionally delete an invoice to Trash and later restore it.

### Journey E — Install a developer update

1. Developer publishes a **versioned, signed GitHub Release** with installer/update artifacts and valid updater metadata.
2. When online, the installed app checks for updates in the background at launch (non-blocking) and provides **Check for Updates** in Settings.
3. If a newer version is available, show a clear Arabic notice with installed version, available version, short release notes, **Update Now** and **Later**.
4. On **Update Now**, ensure ongoing edits are saved; download and verify the update signature, show progress, then install with Tauri's supported Windows update flow (restart if necessary).
5. Reopen with products, invoices, drafts, preferences and database intact. If offline, no update, or download fails, the app continues functioning normally.

---

## 4. Platform and Architecture

| Area | Recommendation | Requirement / Rationale |
|---|---|---|
| Windows shell | **Tauri 2** | A native installable Windows app using the system WebView2 runtime. |
| Native layer | **Rust** | SQLite operations, filesystem, import/export, update integration, safe persistence. |
| UI | **React + TypeScript + Vite** | Modular Arabic-RTL interface following provided reference screenshots. |
| Database | **SQLite** | Per-user local database with schema migrations, transactions, constraints and indexes. |
| Excel input | `calamine` or verified equivalent | `.xlsx` parsing and row-level validation. |
| Excel output | `rust_xlsxwriter` or verified equivalent | `.xlsx` export with correct Unicode and numbers. |
| Updates | `tauri-plugin-updater` v2 | Signed updates via HTTPS / GitHub Releases. |
| Release automation | GitHub Actions + `tauri-action` | Build, sign, attach install/update artifacts and update manifest to tagged releases. |
| Packaging | Windows NSIS `.exe` installer (MSI optional) | Standard Windows setup, shortcuts, uninstall and same-family in-app updates. |

**Important engineering caveats:** Tauri + Rust is not an entirely native-Rust UI; Tauri uses WebView2 on Windows. Low resource consumption and responsiveness are acceptance targets requiring real measurement. RTL PDF/print fidelity must be tested on Windows instead of assumed.

**Data isolation:** Store DB and application assets in Windows per-user application-data directories, **not next to the installed executable**, so installers and updates do not replace data. Never store user data as bundled static assets.

---

## 5. Screens and Information Architecture

1. **New Invoice** — Client/date/invoice ID, item entry, totals, optional notes, save/preview/print/export actions.
2. **Previous Invoices** — Newest first, search by customer name/invoice number, date filter, preview, duplicate, delete.
3. **Products** — Searchable/paginated catalog; create/edit/delete products; import/export Excel.
4. **Trash** — Deleted invoices, restore and separately confirmed permanent deletion.
5. **Invoice Preview** — A clean invoice document independent of editor chrome, with Share, Save PDF, Print, Save Image actions.
6. **Settings** — Company details, SVG brand logo, exact HEX primary color, currency/date preferences, backup/restore, **Updates (current version, check now, update status)**.

### Visual reference

Use the supplied screenshots depicting **Arabic “Create Invoice”** and **“Invoice Preview”** as layout and style references: blue top bar/action buttons, light cream background, white rounded cards, modest borders, clean invoice table, prominent invoice total. Improve information hierarchy and usability without turning it into an unrelated design.

- All on-screen text and user-facing feedback must be Arabic and follow RTL layout.
- Mixed-direction content (invoice ID, phone, date, number, currency) must render correctly.
- Use the **owner-supplied original SVG logo** and **exact HEX brand color** when available; keep both as replaceable assets/design tokens until supplied. Do not infer final values from the screenshots.
- Render logo and seller details in preview, PDF, printed output, and images.
- Support keyboard-only operation, logical Tab order, high-DPI scaling (125–200%), accessible focus states and readable controls.
- The UI should not display every available product at once.

---

## 6. Functional Requirements

### FR-01 — Create and Issue Invoices

- Collect invoice date, customer name and phone manually for each new invoice; optional notes. Do not add customer directory or customer autocomplete.
- Start a new draft with a provisional invoice identifier. Assign a **unique permanent invoice number** (e.g., `INV-000001`) transactionally when the invoice is first issued/saved as final; do not permanently consume numbers for unsaved drafts.
- Enforce at least one valid invoice line before issuing, quantities > 0, unit prices >= 0, reliable totals, and meaningful Arabic validation messages.
- Each line stores item name, quantity, unit price, line total, and optional product ID.
- Recalculate line and invoice totals in the Rust domain layer rather than trusting frontend values.
- Save issued invoices in SQLite, including historical item and seller identity snapshots.
- A saved invoice is retained by default until the user deletes it; reopening it must preserve recorded prices and seller information.

### FR-02 — Select Catalog Products in an Invoice

- Use an accessible searchable combo box inside each item row, **not a full-screen picker**.
- Limit the initial suggestion list (typically 8–10 items) and query SQLite in a paginated/indexed manner. Avoid bringing 10,000+ records to the renderer.
- Support partial Arabic search, normalizing common hamza/diacritic variations for search only (preserve original names for display).
- Show name and base price per result; recently used items may be offered in a small relevant group.
- Arrow keys / Enter / Escape / Tab must work correctly; after selection focus moves to quantity.
- On selection, fill name + base price; calculate total live.
- Provide **Edit Price** on the invoice row; it changes only the current invoice item's price. The stored catalog price must remain unchanged. Allow reset to catalog price.
- Allow adding/removing/reordering item rows while preserving intuitive focus.

### FR-03 — Manual Invoice Items

- Provide a prominent **Add Manual Item** action.
- Let the user enter free-text item name, unit price, and quantity without an existing catalog record.
- Offer an explicit **Save to Products** opt-in, off by default. Do not silently populate the product catalog with temporary invoice items.
- Manual items and catalog-selected items may coexist in one invoice.

### FR-04 — Product Catalog Management

- Add products individually with mandatory name and base price, optional SKU.
- Edit name, price, optional SKU; soft-delete a product if needed. Deleting a product never removes it from previously issued invoice snapshots.
- Support immediate search, sensible sorting, pagination/virtualization, keyboard navigation and empty/loading/error states.
- Validate prices and names and reject malformed input; record audit-friendly created/updated dates where useful.

### FR-05 — Excel Import / Export

- Import `.xlsx` with file picker, worksheet selection and configurable column mapping.
- Support `Product Name`, `Price`, and optional `SKU` columns; provide a downloadable/importable example template.
- Display a **dry-run preview** before altering the database: invalid/missing/negative prices, blank names, duplicate rows in file, possible existing matches, row-specific reasons.
- Let the user choose **skip matched**, **update confirmed matches**, or **add new only**. Prefer SKU as unambiguous matching key when present; name-only matches require review rather than automatic equivalence assumptions.
- Apply confirmed changes transactionally and report imported, updated, skipped and invalid counts.
- Export all/filtered products to `.xlsx` with correct price formats and Arabic Unicode text.

### FR-06 — Invoice History

- Display all issued invoices newest-first; search by customer name or invoice number and filter by date range.
- Allow preview, export/print, and **Duplicate as New**. Duplication creates a new draft with copied line snapshots, a fresh date, and a fresh number upon issue; never modifies the source invoice.
- Allow deletion to **Trash**, restoration, and permanent deletion through a clearly distinct confirmation flow; do not reuse issued numbers after deletion.
- Provide bulk deletion if it remains safe and clear, with strong confirmation and predictable selection.
- Existing invoices retain their saved line prices, product names, seller details and original logo snapshot regardless of subsequent catalog/branding changes.

### FR-07 — Invoice Preview, PDF, Print, Image, Share

- Preview only the invoice document: seller identity/logo, invoice number/date, manual customer details, item table, quantities, prices, totals and notes.
- Use **one canonical invoice snapshot/rendering model** for preview, PDF, print and image, avoiding mismatched calculations.
- **Save PDF** must generate a real PDF to the user's chosen path, defaulting to A4, with Arabic fonts/shaping, RTL, correct SVG resolution, multipage overflow handling, repeated headers where needed. Do not call Windows Print-to-PDF the entire implementation.
- **Print** must use the Windows printer dialog and exclude application toolbar/window chrome.
- **Save PNG/JPG** must capture/render the invoice document alone at high resolution, not a screenshot of the app. Handle long invoices with clear multi-page image behavior; JPG must have an opaque background.
- **Share** an exported document through supported Windows options, or provide a reliable **Open Containing Folder / Copy Path** fallback.
- Permit previewing drafts with a clear Arabic “Draft” indication; final exports must reference a permanently saved/numbered invoice.

### FR-08 — Draft Autosave and Crash Recovery

- Autosave edits to local storage with debounce and durable persistence; avoid freezing the form.
- On re-launch after abnormal close, allow restoring in-progress invoice draft(s), including customer fields, items, quantities, prices and notes.
- Drafts must not silently receive permanent invoice numbers.

### FR-09 — Backup and Restore

- Offer manual backup, auto backup with documented schedule/retention, destination choice, and last successful backup timestamp.
- Include SQLite database and referenced seller-brand/logo assets.
- Create **SQLite-consistent backups** using online backup or WAL-safe technique; never assume copying a live database file alone is consistent.
- Validate a selected restore file, create a pre-restore safety backup, restore transactionally/safely, and re-check integrity. On error retain the active database.
- Warn that backup files on the same physical drive are insufficient against disk loss.

### FR-10 — Settings and Brand Identity

- Seller/company name, phone/contact details and other optional display fields, original SVG logo, exact HEX brand color, default currency (EGP unless user changes it), date format, backup location.
- Persist settings locally and supply reasonable placeholders until authentic assets arrive.
- On finalizing an invoice, snapshot seller fields and logo/branding used by that invoice so later settings changes do not rewrite the archive.

### FR-11 — In-App Auto Update from GitHub Releases (Required)

**User outcome:** The developer releases v1.0.1 after v1.0.0; the existing user sees an in-app Arabic update notice and can click a button to install the update without visiting GitHub, downloading the new installer, or performing a manual reinstall.

#### Release publishing prerequisites

- Configure the official **Tauri 2 updater plugin** and current-compatible Rust/tooling.
- Choose one supported Windows installer/update family (NSIS recommended) and test upgrades between versions.
- Enable generation of signed updater artifacts (`bundle.createUpdaterArtifacts = true` for current Tauri 2 workflows).
- Configure an HTTPS updater endpoint such as `https://github.com/<OWNER>/<REPO>/releases/latest/download/latest.json` (actual owner/repo to be supplied).
- Publish a proper **GitHub Release** with incremented valid semantic version, appropriate signed installer artifact, and updater `latest.json` metadata; **pushing commits to GitHub is not itself a release or update**.
- Automate GitHub Release artifact creation and publication via a controlled GitHub Actions workflow (`tauri-action` or equivalent) that runs on approved version tags/releases, with test gates.
- Protect signing private key/password as CI secrets; place the updater's **public** verification key in the app. Never commit the private signing key, tokens or credentials.
- Tauri updater artifact signature verification is mandatory. Windows Authenticode code signing is distinct and recommended for a smoother Windows trust/SmartScreen experience; it is not a substitute for Tauri's updater signatures.

#### Installed app behavior

- On launch, optionally check online in the background, with non-blocking timeout/retry behavior; provide **Settings > Check for Updates** and current version info. Do not check continuously or interrupt an invoice.
- If no newer version, show **You're up to date** (in Arabic) after manual check; avoid unnecessary popups on every launch.
- If there is a newer release, show version number, release notes, **Update Now**, **Later**, and progress during download/verification/install.
- **Never silently install** an update while the user is composing an invoice. Persist pending edits first and prompt to close/restart as required by the update flow.
- Verify update signatures before installation, reject malformed metadata or invalid signatures, and do not allow unauthorized downgrades.
- If offline, GitHub unavailable, signature invalid, or install fails, show a clear recoverable error and keep the existing application and locally saved data working.
- After successful update, display updated version and preserve products, invoices, drafts, settings and backup configuration. Database schema migrations must be versioned, safe, and backed up before risky updates.
- The application itself does not need continuous internet access, an account or a custom backend.

#### Important platform limitations

- In-app updates still use a signed installer/update mechanism internally; they eliminate **manual user reinstallation**, not installation technology altogether.
- Private GitHub repositories may require authenticated access and are **not assumed to work** with a public `releases/latest/download/latest.json` endpoint. Choose accessible hosting or a secure update service if the release is private.
- Test behavior of Windows admin/UAC permissions and per-user vs machine-wide install; prefer an installer configuration that avoids unnecessary elevation for end users.

---

## 7. Data Model (Proposed)

| Table | Essential fields / semantics |
|---|---|
| `products` | `id`, `name`, `normalized_name`, `sku?`, `base_price_minor`, `deleted_at?`, `created_at`, `updated_at` |
| `invoices` | `id`, `invoice_number UNIQUE NULL for draft`, `invoice_date`, `customer_name`, `customer_phone`, `notes`, `status (draft/issued)`, `seller_snapshot_json`, `total_minor`, `deleted_at?`, timestamps |
| `invoice_items` | `id`, `invoice_id`, `product_id?`, `name_snapshot`, `quantity_decimal`, `unit_price_minor`, `line_total_minor`, `sort_order` |
| `app_settings` | Settings key/value or typed table |
| `invoice_number_sequence` | Durable, transactionally updated counter; do not recycle deleted numbers |
| `schema_migrations` | Applied DB schema versions |
| `backup_history` (optional) | Destination, timestamp, result, file validation metadata |
| `update_preferences` (optional) | Last check, user deferral preference; no billing/tracking data |

**Money and integrity:** Use integer minor currency units (e.g., Egyptian piastres) or a correctly implemented fixed-point decimal. **Do not use binary floating-point** for money. Use consistent quantity precision and rounding policy, transactional final save, parameterized SQL, enabled foreign keys, checks, and schema migrations. Keep issued invoice snapshots independent of live catalog and brand settings. Protect filesystem access with least-privilege Tauri capabilities.

---

## 8. Non-Functional Requirements

### Responsiveness and performance

- Demonstrably usable with at least **10,000 products** and a realistic invoice archive.
- Indexed/paginated search and bounded result lists. Benchmark on Windows hardware, not just development browser.
- No loading full product catalog into the invoice editor or thousands of invoice rows into one DOM tree.
- Responsive keyboard-first flow, minimal modal interruptions, graceful empty/loading/error states.

### Accessibility and localization

- Arabic RTL in screens, confirmation dialogs, updater notices and errors; correct display of mixed RTL/LTR data.
- Logical focus order, accessible labels, visible focus indicators, contrast, controls usable at common Windows display scaling.

### Security and privacy

- Invoices and customer details remain local; update checks transmit only what the updater needs to request a release, not the contents of invoices.
- Safe SQL, validation, escaped user input, least-privilege filesystem permissions and safe SVG handling.
- Signed updates, protected CI secrets, HTTPS metadata, no automatic trust in random GitHub assets.
- Recoverable failure behavior: failed export/import/update must not damage stored records.

### Availability and offline behavior

- All invoice workflows (create, search, save, PDF, print, image, backups) work offline, subject to local printer/runtime availability.
- Updates are supplemental online functionality and must never block app launch when disconnected.

---

## 9. Delivery Plan

**Phase 1 — Foundation:** Tauri 2 app, React/TypeScript, Arabic RTL design system, navigation, SQLite schema/migrations, per-user app data, app configuration.

**Phase 2 — Product catalog:** CRUD, indexed Arabic search, keyboard combobox, Excel XLSX preview/import/export, import reporting.

**Phase 3 — Invoice editor:** Manual customers and items, product selection, line-only price override, totals, drafts, safe transactional numbering, persistent invoice snapshots.

**Phase 4 — History and protection:** Invoice history/filtering, duplicate-as-new, Trash/restore, backup/restore, branding snapshots.

**Phase 5 — High-fidelity output:** Preview, real PDF, Windows print, PNG/JPG with Arabic RTL/SVG/multipage Windows tests.

**Phase 6 — Release and update pipeline:** NSIS installer, signing keys and GitHub Actions Secrets, GitHub Releases workflow, signed updater JSON, Settings > Updates, interactive update flow and two-version end-to-end upgrade test.

**Phase 7 — Hardening and delivery:** Database migration rehearsal, E2E/performance tests, accessibility checks, user docs, Windows installer and rollback/restore documentation.

---

## 10. Acceptance Criteria / Definition of Done

1. Install and run the genuine Windows app on a clean Windows 10/11 machine with required WebView2 runtime available/provisioned.
2. Create a correct invoice using a mix of catalog and manual items; change one line's price without modifying the catalog.
3. Save a final invoice, reopen after app restart, and verify item/seller/brand snapshots and totals are unchanged after product edit/delete or brand change.
4. Add at least 10,000 products; benchmark fluid Arabic search, keyboard selection and history navigation without rendering all items.
5. Preview an XLSX import containing blank/duplicate/invalid rows and verify correct reporting plus rollback-safe confirmed import.
6. Export products to XLSX and verify Arabic text and numeric prices in spreadsheet software.
7. Recover an unfinished invoice draft after abrupt app termination.
8. Search history by customer/date/number, duplicate as new with a new number, delete to Trash and restore safely.
9. Export and inspect a real PDF, PNG, JPG and Windows print output with Arabic text, SVG logo, correct totals, multi-page cases, and no app buttons/window chrome.
10. Produce and restore a validated backup, reject corrupt backups without damaging current records.
11. Publish **two consecutive signed versions** (for example v1.0.0 then v1.0.1) to a test GitHub Releases feed; verify notification, release notes, Update Now, download progress, signature validation, install/restart and preserved user data.
12. Verify **Later**, offline/no-update, invalid signature, interrupted download and failed-install behavior without data loss; test no repeated forced notification.
13. Produce the full source code, DB migrations, tests, sample XLSX import template, test fixtures, GitHub Actions release workflow, Windows installer, build and update instructions. State any Windows-specific steps not tested if test access was unavailable.

---

## 11. Inputs / Decisions Before Public Release

- Original company **SVG logo** (not a screenshot or raster approximation).
- Exact **HEX** brand primary color.
- Company name, phone, optional address/other invoice footer content; confirmation of final invoice template if different from screenshots.
- GitHub repository owner/name and whether releases will be publicly accessible.
- Secure, developer-controlled updater private signing key and the associated public key (generated locally/securely; do not send private key in chat).
- Whether Windows Authenticode signing certificate will be used (recommended; separate from updater signatures).
- Final choice of installer scope (per-user or machine-wide) and Windows test machines.

These inputs must not block implementing configurable placeholders or the rest of the application.

---

## 12. Implementation Notes and References

This PRD describes **behavior and acceptance criteria**, not a claim that any implementation has already been built or tested.

- Tauri 2 updater plugin: https://v2.tauri.app/plugin/updater/
- Tauri official GitHub Actions distribution guide: https://v2.tauri.app/distribute/pipelines/github/
- Branding/layout references: the owner's two screenshots of **Create Invoice** and **Invoice Preview**, supplied in this project conversation.

**Change log**  
- **1.0:** Consolidated invoicing, product management, Excel, Arabic RTL UX, snapshots, drafts, history, backup, and export requirements.  
- **1.1:** Added signed, user-initiated in-app updates via GitHub Releases with release automation, migration/data safety and failure-path tests; document language changed to English.
