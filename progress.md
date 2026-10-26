# ASTER --- Development Progress

> **Project:** ASTER Medical Wholesale Billing ERP\
> **Stack:** Rust + `egui`/`eframe` + SQLite (`rusqlite`)\
> **Last updated:** 2026-10-26 (Stock screen and inventory filters update)\
> **Purpose:** Track completed work, current status, planned phases, and
> module responsibilities.
>
> **Status convention:** `[x]` confirmed by project details/screenshots;
> `[~]` partially implemented or visible but not yet verified; `[ ]`
> planned/not yet confirmed.\
> This document reflects the progress reported so far. A checkbox does
> not claim hidden persistence or business logic works unless it has
> been tested.

------------------------------------------------------------------------

## 1. Project Principles

-   [x] Build a fresh ASTER application rather than importing legacy MDB
    records.
-   [x] Use Rust and native `egui`/`eframe` for a lightweight Windows
    desktop application.
-   [x] Use SQLite through `rusqlite` with the `bundled` feature; no
    separate SQLite server is required.
-   [x] Keep the current native OS window decorations.
-   [x] Use Git and the remote `origin/main` for source history.
-   [ ] Confirm automated backup and database restore procedures.
-   [ ] Confirm all business calculations and database operations with
    automated tests.
-   [x] Record final-release requirement: ASTER business data must live
    inside the chosen ASTER application folder, not permanently depend
    on `%LOCALAPPDATA%`. Implementation is deferred until packaging so
    the current working development database paths are not changed
    prematurely.

## 2. Environment and Toolchain

-   [x] Rust toolchain installed.
-   [x] GNU Rust toolchain installed: `stable-x86_64-pc-windows-gnu`.
-   [x] GNU toolchain set as default, according to user report.
-   [x] `eframe`/`egui` dependency compilation check succeeded with the
    GNU toolchain.
-   [x] Native desktop UI launched successfully with the GNU toolchain,
    according to user report.
-   [x] Git repository connected to `origin/main`; the reported branch
    was up to date with the remote.
-   [x] Visual Studio Build Tools avoided due to limited disk space.
-   [x] `cargo fmt` completed successfully after formatting the changed
    Rust files (user report, 2026-10-20).
-   [x] `cargo check` passed on the current project state (user report,
    2026-10-20).
-   [x] `cargo test` passed on the current project state (user report,
    2026-10-20); see Phase 2 for the database tests added in
    `database.rs`.
-   [ ] Confirm a clean `cargo run` after the database foundation
    change.
-   [ ] Run `cargo clippy` on the current project state.
-   [ ] Document exact Rust toolchain and build commands in the project
    README/DEVELOPMENT guide.

## 3. Dependencies Confirmed in `aster/Cargo.toml`

-   [x] `eframe = "0.33"` --- native application framework.
-   [x] `egui = "0.33"` --- immediate-mode UI.
-   [x] `image = "0.25.10"` with PNG support --- image asset loading.
-   [x] `argon2 = "0.5"` --- password hashing dependency.
-   [x] `password-hash = "0.5"` with `getrandom` --- password-hash
    support.
-   [x] `rusqlite = "0.38"` with `bundled` --- SQLite integration and
    bundled SQLite library.
-   [x] `keyring = "3"` --- OS credential-store integration dependency.
-   [ ] Verify each dependency is actually used correctly in runtime
    code.
-   [ ] Confirm `Cargo.lock` is committed after reviewing the staged
    changes.

## 4. Application Flow and UI

### 4.1 Date and Financial-Year Selection

Module: `src/screens/year_date.rs`; supporting date module:
`src/date.rs`.

-   [x] A working-period selection screen is visible.
-   [x] Financial years are displayed as selectable options.
-   [x] Current financial year shown in the screenshot: `2026–2027`.
-   [x] Working date is displayed in `DD/MM/YYYY` format in the
    screenshot.
-   [x] Calendar navigation and date selection UI are visible.
-   [x] A "Today" control is visible.
-   [x] UI states the financial year runs from 1 April to 31 March.
-   [x] Continue action leads to the sign-in screen, as shown.
-   [ ] Verify date input validation and boundary handling in code.
-   [ ] Verify a selected date must fall within the selected financial
    year.
-   [ ] Verify financial-year selection is saved and restored
    appropriately.
-   [ ] Verify adding the next financial year does not create duplicates
    or invalid ranges.

### 4.2 Sign-In Screen

Module: `src/screens/login.rs`.

-   [x] A sign-in screen is visible.
-   [x] Username field is displayed.
-   [x] Password field is displayed with a visibility-toggle control.
-   [x] "Remember my password on this PC" option is visible.
-   [x] Sign-in and Back controls are visible.
-   [x] The screen displays the selected financial year and working
    date.
-   [x] `argon2`, `password-hash`, and `keyring` dependencies are
    declared.
-   [x] Sign-in was tested successfully after replacing
    `src/database.rs` (user report, 2026-10-20).
-   [x] Database-backed credential verification and password-hash
    support are implemented in the current database module; full
    security edge-case review remains open.
-   [ ] Verify "remember password" uses Windows Credential
    Manager/keyring correctly and only after user consent.
-   [ ] Verify incorrect credentials, empty fields, locked accounts, and
    sign-out behaviour.
-   [x] Verify first-run administrator creation and recovery process.

### 4.3 Main Workspace and Dashboard

Module: `src/screens/workspace.rs`.

-   [x] Main workspace/dashboard is visible.
-   [x] ASTER branding and user/profile area are visible.
-   [x] Top tab strip is visible with Dashboard selected.
-   [x] Sidebar navigation is visible.
-   [x] Sidebar sections shown: Workspace, Masters, Transactions,
    Accounts & Reports, System.
-   [x] Navigation entries shown: Overview, Company settings, Customers,
    Suppliers, Products/Medicines, Purchase, Sales invoice, Stock,
    Accounts, Reports, Settings.
-   [x] Dashboard summary cards are visible: Today's sales, Stock
    alerts, Receivables, Expiring batches.
-   [x] Recent activity panel is visible.
-   [x] Quick actions are visible: New sales invoice, Record purchase,
    Add product, Check stock, Open reports.
-   [x] Status bar displays "Database connected" and the selected
    financial year/date in the screenshot.
-   [x] Company Settings, Customers, Suppliers, Products/Medicines, Purchase, and Sales Invoice are routed to dedicated screen modules.
-   [x] Stock is now routed to a dedicated stock overview screen.
-   [x] Company Settings, Customers, Suppliers, Products/Medicines, Purchase, Sales Invoice, and Stock are routed to dedicated screen modules.
-   [~] Accounts, Reports, and System Settings remain navigation entries without confirmed dedicated functional implementations.
-   [ ] Verify tab switching, closing tabs, duplicate-tab prevention, and unsaved-change warnings.
-   [ ] Verify tab switching, closing tabs, duplicate-tab prevention,
    and unsaved-change warnings.
-   [~] Dashboard cards and recent activity UI are present; the supplied source shows placeholder-style values (for example, em-dash metrics), so database-backed dashboard aggregation is not confirmed.
-   [ ] Verify the "Database connected" status is based on an actual
    successful connection and query.

## 5. Source Modules in the Uploaded `screens.zip`

| Module/file | Current status from source inspection | Responsibilities / gaps |
|---|---|---|
| `src/database.rs` | Updated; contains schema migrations and master/transaction APIs | Global user/year registry; year-scoped company, customer, supplier, manufacturer, unit, product, purchase, batch, stock movement, sales invoice, batch allocation and payment tables. Purchase/sales posting methods exist. Stock query APIs, account ledgers/receipts/payments, report queries, and system-preference APIs are not present in the inspected method list. |
| `src/screens/mod.rs` | Updated | Declares Company Settings, Customer Master, Supplier Master, Product Master, Purchase Entry and Sales Invoice modules. |
| `src/screens/company_settings.rs` | Implemented screen | Company legal/trade details, GST/drug licences, address/contact, invoice prefix and bank details; uses database load/save methods. Restart/year-switch persistence still needs runtime verification. |
| `src/screens/customer_master.rs` | Implemented screen | Customer list/search and editor; database supports create/update and active/inactive status. Test cases remain outstanding. |
| `src/screens/supplier_master.rs` | Implemented screen | Supplier details, list/editor, manufacturer and product relationships; database supports save/update, supplier relationships and active/inactive status. Test cases remain outstanding. |
| `src/screens/product_master.rs` | Implemented screen | Product list/search, manufacturer/unit selectors, edit and active/inactive actions; database supports product CRUD and dropdown data. Test cases and visual consistency review remain. |
| `src/screens/purchase_entry.rs` | Implemented entry/posting screen | Purchase draft UI and posting call; database posting writes purchase document/items, batches, stock quantities and stock-movement records. Runtime acceptance tests and purchase history/return workflow remain. |
| `src/screens/sales_invoice.rs` | Implemented invoice entry/posting screen | Customer/product selection, invoice lines/totals and posting; date helper converts Indian `DD/MM/YYYY` input to validated ISO date. Database posting saves invoice/items, allocates and deducts batch stock, records stock movements and received payment. Invoice history, preview, print/PDF and returns are not confirmed implemented. |
| `src/screens/stock.rs` | Implemented stock overview screen; latest code was reported working before filter restoration | Loads batch records through `db.stock_batches(year, search)`, aggregates stock by product, shows product count/on-hand quantity/stock valuation, supports product selection and a batch detail panel with expiry, free quantity, MRP and sale rate. All/Low/Expiring/Expired/Out-of-stock filters have now been restored in the current source. Column sorting and stock movement history are not implemented/confirmed. Re-run `cargo check` after the latest filter edits. |
| `src/screens/workspace.rs` | Updated routing and workspace | Sidebar includes Stock, Accounts, Reports and System Settings. Company, Customer, Supplier, Product, Purchase and Sales screens are routed to dedicated modules. Accounts, Reports and System Settings remain to be implemented; dashboard metrics are not yet confirmed database-driven. |
| `src/screens/year_date.rs`, `src/screens/login.rs` | Present | Financial-year/date flow and sign-in; continue validation of edge cases and credential persistence. |
| `src/app.rs`, `src/date.rs`, `src/theme.rs`, `src/logging.rs`, `src/main.rs` | Expected project modules; not included in this ZIP | Their latest exact contents could not be reviewed from this archive. |

**Evidence rule:** Source presence and API inspection do not prove runtime correctness. Items marked implemented still need targeted testing unless a user-reported test is explicitly noted.

## 6. Git / Current Working State

Latest reported `git status` was run from `D:\PROJECT\ASTER`:

-   [x] Branch: `main`.
-   [x] Local branch reported up to date with `origin/main`.
-   [x] These six files were staged for commit at the time of the
    report:
    -   `aster/Cargo.lock`
    -   `aster/Cargo.toml`
    -   `aster/src/date.rs`
    -   `aster/src/screens/login.rs`
    -   `aster/src/screens/workspace.rs`
    -   `aster/src/screens/year_date.rs`
-   [ ] Review staged diff before committing.
-   [ ] Commit the current working milestone with a clear message.
-   [ ] Create the next feature branch after the current work is
    committed.
-   [ ] Push each reviewed milestone to the remote.

Suggested review commands:

``` powershell
git diff --cached --stat
git diff --cached
```

Do not reset or discard staged changes without reviewing them.

## 7. Development Phases

### Phase 0 --- Environment and Initial Native Window

-   [x] Rust project created.
-   [x] `eframe`/`egui` configured.
-   [x] GNU toolchain build check passed.
-   [x] Native window launched.
-   [x] Git remote push completed, according to user report.

**Phase status:** Complete for the initial scaffold.

### Phase 1 --- Application Shell and Entry Flow

-   [x] Financial-year/date selection UI.
-   [x] Sign-in UI.
-   [x] Main workspace/dashboard UI.
-   [x] Sidebar and navigation categories.
-   [x] Dashboard cards and quick actions.
-   [x] Native window title bar retained.
-   \[\~\] Authentication, date/year selection, and navigation behaviour
    need code-level verification.
-   [ ] Finish tab lifecycle and screen-routing tests.
-   [ ] Confirm UI behaviour on different window sizes and scaling
    settings.

**Phase status:** UI is implemented to the point shown in screenshots;
functional completeness is not yet verified.

### Phase 2 --- SQLite Foundation

-   [x] `rusqlite` dependency declared with `bundled`.
-   [x] Global database is created at
    `%LOCALAPPDATA%\Aster\aster.sqlite3`.
-   [x] Application data root is created at
    `%LOCALAPPDATA%\Aster\data\`.
-   [x] Separate SQLite database files are created per financial year at
    `data\YYYY-YYYY\aster.sqlite3`.
-   [x] Runtime filesystem inspection confirmed the global database,
    three year databases (`2026-2027`, `2027-2028`, `2028-2029`), and
    `logs\aster.log` exist (user report, 2026-10-20).
-   [x] Global schema has a version number and migration tracking
    (`PRAGMA user_version`, `schema_migrations`).
-   [x] Financial-year database has schema-version tracking and metadata
    identifying its financial-year boundaries.
-   [x] Financial-year creation initializes its separate database and
    rejects a duplicate year.
-   [x] Financial-year validation rejects out-of-range start years.
-   [x] SQLite connection configuration includes foreign-key
    enforcement, a 5-second busy timeout, WAL journal mode, and
    synchronous configuration.
-   [x] Database tests are included for initial database/year creation,
    separate database creation and duplicate rejection, and invalid
    financial-year validation.
-   [x] User reports `cargo fmt`, `cargo check`, and `cargo test` all
    pass after replacing only `src/database.rs` (2026-10-20).
-   [x] User reports sign-in still works after the database module
    replacement (2026-10-20).
-   [x] No old MDB data was imported as part of this change; legacy
    files remain design references only.
-   [ ] Verify persistence after closing and reopening the application,
    including re-opening existing financial-year databases.
-   [ ] Inspect/test rollback and failure behaviour for all multi-step
    database operations.
-   [ ] Add safe backup and restore procedures.
-   [ ] Add actual business tables to each financial-year database in a
    later schema migration.

**Phase status:** Database initialization foundation implemented and
runtime paths verified. The user confirmed formatting, compilation,
tests, and sign-in succeed. Business tables and full
persistence/rollback acceptance tests are still pending.

### Phase 3 --- Company Setup, Financial Year, and Authentication

-   [x] Financial-year/date UI and sign-in UI exist.
-   [x] Company Settings screen reads and saves year-scoped company details.
-   [x] Database uses Argon2 password hashing for administrator credentials.
-   [~] User/password and remembered-credential edge cases need verification.
-   [ ] Verify company settings survive restart and remain isolated by financial year.
-   [ ] Implement/review role and permission checks, recovery, and audit events.

**Phase status:** Core UI and company settings persistence API exist; security and restart/year-isolation acceptance tests remain.

### Phase 4 --- Master Data

-   [x] Company Settings UI and database save/load methods.
-   [x] Customer Master screen and database create/update/search/status methods.
-   [x] Supplier Master screen and database save/search/status methods.
-   [x] Supplier-manufacturer and supplier-product relationship tables/APIs.
-   [x] Product/medicine Master screen and database save/search/status methods.
-   [x] Manufacturer and unit lookup data, HSN, GST, schedule, barcode and reorder-level fields.
-   [~] UI-level validation and database constraints are present in places; exhaustive field/duplicate validation testing remains.
-   [ ] Test create, edit, search, deactivate/reactivate, duplicate handling and reload for all master screens.
-   [ ] Verify master data persists after restart and is scoped to the selected financial year.
-   [ ] Review UI scaling, keyboard navigation, error display and theme consistency.

**Phase status:** Company, customer, supplier and product master screens are implemented and routed. Do not mark their end-to-end acceptance complete until tested.

### Phase 5 --- Purchase and Inventory Foundation

-   [x] Purchase Entry screen exists and calls the purchase-posting database method.
-   [x] Schema includes purchase counters/documents/items, batches, stock movements and supplier relationships.
-   [x] Purchase posting writes purchase document and line snapshots, updates batch quantities, and records stock movements in a database transaction.
-   [~] Purchase form includes supplier/product and batch/expiry/quantity/rate/tax-related fields; all edge cases need runtime testing.
-   [ ] Verify atomic rollback on invalid line, database error, and stock/batch update failure.
-   [ ] Implement purchase history/detail, correction/return workflows and supplier invoice duplicate policy.
-   [x] Implement Stock overview UI using `db.stock_batches(year, search)`; aggregate batch data into product rows.
-   [x] Add product count, low-stock count, on-hand quantity, and stock valuation summary metrics.
-   [x] Add selectable product rows and a batch breakdown showing quantity, free quantity, expiry, MRP, and sale rate.
-   [x] Restore filters for All inventory, Low stock, Expiring in 90 days, Expired, and Out of stock.
-   [x] Add date-window helpers for the 90-day expiry filter and expiry labels in the product report.
-   [~] Search, refresh, and the stock overview were reported working; re-run `cargo check` and smoke-test all filters after the latest edits.
-   [ ] Restore/add column sorting controls from the earlier iteration; the supplied older reference contains filters but no sorting implementation.
-   [ ] Add stock movement history and controlled stock adjustments with audited movement records.
-   [ ] Add stock-query and inventory reconciliation tests; reconcile displayed stock against batch and movement records.

**Phase status:** Purchase posting and stock/batch ledger foundation exist. A stock overview with product aggregation, batch details, and five filters is implemented in the current working source. Sorting, movement history, adjustment workflow, and end-to-end inventory verification remain open.

### Phase 6 --- Sales Invoice and GST

-   [x] Sales Invoice entry screen exists and is routed from the workspace.
-   [x] Invoice date UI uses Indian `DD/MM/YYYY`; the local conversion helper validates the date and converts it to ISO `YYYY-MM-DD` for posting.
-   [x] Database schema includes sales counters, invoices, invoice items, batch allocations, and payment records.
-   [x] Sales posting method validates the draft, saves invoice snapshots, allocates/deducts batch stock, records stock movements and records the amount received.
-   [~] The screen shows a successful-posting message and clears the form after success; this is posting confirmation, not proof of print/PDF generation.
-   [ ] Test CGST/SGST versus IGST, discounts, rounding, partial/full receipt, insufficient stock, expired batches, duplicate numbering and rollback.
-   [ ] Review authoritative money calculations and ensure floating-point UI values do not become the source of truth for persisted totals.
-   [ ] Add saved invoice history/detail and reopen/preview functionality.
-   [ ] Implement sales returns/credit notes and reversal rules as required.

**Phase status:** Basic invoice entry and posting exist. Printing/PDF, history, returns, and full accounting reconciliation remain incomplete or unverified.

### Phase 7 --- Printing and PDF

-   [ ] Select and test a Rust-compatible PDF library.
-   [ ] Create invoice layout using saved company and invoice snapshots.
-   [ ] Add print preview, PDF export and printer/paper-size handling.
-   [ ] Test long names, multi-page invoices, totals and GST display.

**Phase status:** Not confirmed implemented.

### Phase 8 --- Accounts and Ledgers

-   [~] Customer/supplier opening-balance and credit-limit fields exist in master-data schema.
-   [~] Sales invoice received amount/payment mode is stored; supplier purchase documents exist.
-   [ ] Implement Accounts screen and database APIs for customer receipts and supplier payments.
-   [ ] Implement customer/supplier ledgers, outstanding balances, allocations/settlements and opening-balance handling.
-   [ ] Define correction/reversal rules and preserve consistency with posted source documents.
-   [ ] Add account reconciliation and transaction rollback tests.

**Phase status:** Foundational fields and transaction payment data exist, but a dedicated Accounts workflow and ledger queries are not implemented in the supplied source.

### Phase 9 --- Reports

-   [ ] Implement Reports screen and parameter/filter controls.
-   [ ] Add sales/purchase summaries and product-wise sales reports.
-   [ ] Add current stock, stock movement, batch and expiry reports.
-   [ ] Add customer/supplier ledger and outstanding reports.
-   [ ] Add receipt/payment and GST/HSN summaries.
-   [ ] Support date and financial-year filters, export/print and reconciliation tests.

**Phase status:** Navigation entry exists, but no dedicated reports implementation or report-query API is confirmed in the uploaded source.

### Phase 9A --- System Settings (distinct from Company Settings)

-   [x] Sidebar entry for System Settings exists.
-   [ ] Implement application preferences and persist/reload them.
-   [ ] Define invoice numbering defaults, tax defaults and relevant business configuration without duplicating Company Settings.
-   [ ] Implement database backup/restore with validation and safe failure handling.
-   [ ] Decide configurable data location and logging preferences; document migration behaviour.

**Phase status:** Navigation only; a dedicated system-settings screen is not confirmed implemented.

### Phase 10 --- Reliability and Release

-   [ ] Final-release data layout: store data inside the chosen ASTER
    application folder. Target layout:

    ``` text
    ASTER/
    ├── Aster.exe
    ├── assets/
    ├── data/
    │   ├── aster.sqlite3
    │   ├── 2026-2027/aster.sqlite3
    │   └── 2027-2028/aster.sqlite3
    ├── logs/
    └── backups/
    ```

-   [ ] Make the data directory configurable or derive it from the
    selected installation/data location; do not hard-code the developer
    machine path.

-   [ ] Ensure the selected folder is writable; avoid assuming
    `C:\Program Files` is writable by a standard user.

-   [ ] Add a safe one-time migration/copy plan from
    `%LOCALAPPDATA%\Aster` for an existing installation, preserving
    original files until verification succeeds.

-   [ ] Ensure upgrades never overwrite user databases and run schema
    migrations instead.

-   [ ] Test launching from a different folder and on a clean Windows
    account.

-   [ ] Tested database backup and restore.

-   [ ] Startup recovery and clear database-error handling.

-   [ ] Audit logging and permissions review.

-   [ ] Performance checks with realistic data volumes.

-   [ ] Full regression test suite.

-   [ ] Installer/package tested on a clean Windows environment.

-   [ ] Versioning and upgrade/migration strategy.

-   [ ] User documentation and recovery instructions.

-   [ ] Final acceptance testing with the business owner.

**Phase status:** Planned.

## 8. Immediate Next Actions

1.  [x] Review the uploaded `screens.zip`, including the current screen modules and `database.rs`.
2.  [x] Update this progress tracker to reflect the current master, purchase and sales implementation visible in the uploaded source.
3.  [ ] Run `cargo fmt`, `cargo check`, `cargo test`, and `cargo run` against the exact current working tree.
4.  [ ] Smoke-test company settings, customer, supplier and product create/edit/search/status changes, including restart persistence.
5.  [ ] Smoke-test purchase posting and reconcile batch quantities/stock movements.
6.  [ ] Smoke-test sales invoice posting: date conversion, tax calculations, received amount, batch stock deduction, and rollback on failure.
7.  [x] Implement the Stock overview using `db.stock_batches()` and add the five inventory filters (All, Low stock, Expiring in 90 days, Expired, Out of stock).
8.  [ ] Re-run `cargo fmt`, `cargo check`, `cargo test`, and `cargo run` against the latest combined working tree; verify the stock filters and check for warnings.
9.  [ ] Restore column sorting from the earlier iteration, once its original sorting options/behaviour are identified.
10. [ ] Add stock movement history and controlled adjustment workflows with tests.
11. [ ] Implement Accounts and receipt/payment/ledger APIs, then Reports using verified database queries.
12. [ ] Implement System Settings separately from Company Settings.
13. [ ] Add invoice history/preview/print/PDF after core transaction workflows are tested.
14. [ ] Review `git diff`, run regression tests, commit the reviewed milestone, and update this file again.

## 9. Definition of "Complete"

A screen is not considered complete merely because it renders. Mark a
feature complete only when:

-   It validates user input.
-   It persists data where required.
-   It reloads correctly after restarting the application.
-   It handles invalid input and database errors safely.
-   Its business rules have tests.
-   It has no fake success messages or nonfunctional controls.
-   Changes are reviewed, tested, and committed.

------------------------------------------------------------------------

**Current overall assessment:** The uploaded source contains the application shell plus dedicated Company Settings, Customer Master, Supplier Master, Product Master, Purchase Entry, and Sales Invoice screens. `database.rs` includes year-scoped master-data APIs and purchase/sales posting methods that update batch stock and movement records. Stock, Accounts, Reports, and System Settings remain the next major screens; invoice history and print/PDF are also not confirmed implemented. Previously reported `cargo fmt`, `cargo check`, `cargo test`, and sign-in success apply to an earlier project state and must be rerun against the current working tree. The final-release requirement to keep data inside the ASTER application folder is documented but not implemented; current development paths remain unchanged by design.

------------------------------------------------------------------------

## Latest Update --- 2026-10-26

### Source reviewed

The uploaded `screens.zip` contains `screens/` modules for Company Settings, Customer Master, Supplier Master, Product Master, Purchase Entry, Sales Invoice, the workspace, sign-in, and year/date selection, plus the current `database.rs` and this progress file.

### Confirmed from source inspection

-   [x] Company Settings uses year-scoped database load/save methods.
-   [x] Customer, supplier and product master screens are present and have corresponding database methods for their primary CRUD/status operations.
-   [x] Purchase schema and posting method cover purchase documents/items, batches, stock quantities and stock-movement entries.
-   [x] Sales invoice schema and posting method cover invoice/item snapshots, batch allocation and stock deduction, stock movements, and a received-payment record.
-   [x] Sales invoice screen contains a local Indian-date-to-ISO conversion helper.
-   [x] Workspace sidebar exposes Stock, Accounts, Reports and System Settings.
-   [ ] Dedicated Stock, Accounts, Reports and System Settings screens/APIs are not confirmed in this archive; these are the remaining primary screens.
-   [ ] Invoice history, print preview and PDF export are not confirmed implemented.

### Implementation order

1. **Stock** — add read-only stock overview, batch details, expiry/low-stock alerts and movement history; add tested database query methods first. Then consider adjustments with audited movement records.
2. **Accounts** — receipts, supplier payments, customer/supplier ledgers and outstanding balances.
3. **Reports** — sales/purchase, stock/expiry, outstanding, receipt/payment and GST/HSN reports using verified query APIs.
4. **System Settings** — application preferences, backup/restore and configurable data-location behaviour.
5. **Sales completion** — invoice history, preview, printing/PDF, and return/reversal workflows.

**Important:** The archive has been reviewed and the progress tracker updated. Runtime compilation/tests were not run against the user's local working tree as part of this documentation update. The tracker deliberately distinguishes code present from functionality verified in a running application.


------------------------------------------------------------------------

## Latest Working-Tree Update --- 2026-10-26 (Stock Screen and Filters)

This update supplements the earlier archive/source-review notes above. It does not erase previous milestones or treat pending tests as complete. The user reported that the integrated stock overview worked before the filter restoration.

### Completed in the current stock-screen implementation

- [x] Wired the stock screen to `db.stock_batches(app.selected_year, &state.search)` instead of using a disconnected/mock stock list.
- [x] Aggregated active batch records into product-level rows for the main stock table.
- [x] Added/retained search and Refresh controls.
- [x] Added product, low-stock, and stock-valuation summary metrics.
- [x] Added selectable product rows and a product report/detail panel.
- [x] Added batch breakdown fields for batch number, expiry, paid quantity, free quantity, MRP, and sale rate.
- [x] Restored the five filter choices: All inventory, Low stock, Expiring in 90 days, Expired, and Out of stock.
- [x] Added date-window calculation and month-length/leap-year helper functions for the expiry window.
- [x] Corrected the filter integration instructions so `visible_rows` is built inside `draw_table()` and the table loop and empty-state check use that filtered collection.
- [x] Identified and removed the unnecessary `HashSet` import and duplicate unused `visible_rows` block that caused compiler warnings.

### Still open for Stock

- [ ] Run `cargo check` after the final filter edits and confirm no warnings remain.
- [ ] Run `cargo fmt` and the test suite against the current combined working tree.
- [ ] Test each filter with representative data, including no expiry, expiry today, expiry within 90 days, expired batches, zero stock, and product totals across multiple batches.
- [ ] Restore the column sorting controls from the earlier version; the reference file supplied in this conversation contains filter buttons but does not show sorting controls, so the exact previous sorting behaviour still needs to be identified.
- [ ] Add stock movement history and controlled adjustment workflows.
- [ ] Reconcile stock valuation and quantities against purchase posting and stock movement records.

### Current next implementation priorities

1. **Finish and verify Stock** — build/test the filter changes, then restore sorting and add movement history/adjustments.
2. **Accounts** — receipts, supplier payments, customer/supplier ledgers, allocations, and outstanding balances.
3. **Reports** — sales/purchase, stock/expiry, ledger/outstanding, receipts/payments, and GST/HSN reports.
4. **System Settings** — application preferences, backup/restore, and data-location settings.
5. **Sales completion** — saved invoice history, preview, printing/PDF, returns, and reversal rules.
6. **Reliability/release** — regression tests, backups, packaging, safe upgrades, and keeping business data in the chosen ASTER application folder.

**Current overall assessment (2026-10-26):** The application shell and dedicated Company Settings, Customer Master, Supplier Master, Product Master, Purchase Entry, Sales Invoice, and Stock screens are present in the working project/source history. The Stock screen now uses the database batch query, shows product-level inventory and batch details, and includes the five restored filters. The user previously reported that the stock integration worked; the final filter edits still need a fresh build and filter-by-filter smoke test. Column sorting, Accounts, Reports, System Settings, invoice history/print/PDF, returns, backup/restore, and final packaging remain outstanding. Earlier progress is retained above; this update changes the current status where newer work supersedes the older Stock-not-implemented notes.
