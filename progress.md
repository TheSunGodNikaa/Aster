# ASTER --- Development Progress

> **Project:** ASTER Medical Wholesale Billing ERP\
> **Stack:** Rust + `egui`/`eframe` + SQLite (`rusqlite`)\
> **Last updated:** 2026-10-21\
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
-   [ ] Verify first-run administrator creation and recovery process.

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
-   [ ] Verify navigation entries open functional screens rather than
    placeholders.
-   [ ] Verify tab switching, closing tabs, duplicate-tab prevention,
    and unsaved-change warnings.
-   [ ] Verify dashboard figures are database-driven rather than
    placeholders.
-   [ ] Verify the "Database connected" status is based on an actual
    successful connection and query.

## 5. Source Modules Currently Reported

The following files were listed in the current project:

  ----------------------------------------------------------------------------
  Module/file                  Current status          Known responsibility /
                                                       next verification
  ---------------------------- ----------------------- -----------------------
  `src/main.rs`                Present                 Application entry
                                                       point; inspect startup
                                                       and error handling.

  `src/app.rs`                 Present                 Application state and
                                                       top-level routing;
                                                       inspect lifecycle and
                                                       screen state.

  `src/database.rs`            Updated; runtime        Global SQLite database,
                               initialization verified financial-year
                                                       registry, per-year
                                                       SQLite database
                                                       initialization,
                                                       schema-version
                                                       tracking, connection
                                                       configuration, and
                                                       database tests.

  `src/date.rs`                Present,                Date and financial-year
                               modified/staged         logic; inspect
                                                       validation and tests.

  `src/logging.rs`             Present                 Logging setup; verify
                                                       file paths, log
                                                       rotation, and error
                                                       reporting.

  `src/theme.rs`               Present                 Shared UI theme; verify
                                                       consistency and
                                                       accessibility.

  `src/screens/mod.rs`         Present                 Screen module
                                                       declarations.

  `src/screens/year_date.rs`   Present,                Working-date and
                               modified/staged         financial-year screen.

  `src/screens/login.rs`       Present,                Sign-in screen and
                               modified/staged         authentication UI.

  `src/screens/workspace.rs`   Present,                Main workspace,
                               modified/staged         sidebar, dashboard and
                                                       navigation UI.
  ----------------------------------------------------------------------------

**Important:** The file list confirms that modules exist; it does not by
itself confirm their internal implementations or correctness.

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

-   [x] Financial-year/date UI exists.
-   [x] Sign-in UI exists.
-   [x] Authentication-related dependencies are declared.
-   [ ] Persist company profile.
-   [ ] Define financial-year records and validation rules.
-   [ ] Implement secure first-run administrator setup.
-   [ ] Persist users and password hashes.
-   [ ] Implement role/permission checks.
-   [ ] Implement safe remembered-credential behaviour.
-   [ ] Add audit events for security-sensitive actions.

**Phase status:** UI foundation exists; data-backed behaviour remains
unverified/planned.

### Phase 4 --- Master Data

-   [ ] Company settings persistence and reload.
-   [ ] Customer creation, search, edit, and deactivate.
-   [ ] Supplier creation, search, edit, and deactivate.
-   [ ] Product/medicine master.
-   [ ] Manufacturer, unit/pack configuration, HSN and tax-rate fields.
-   [ ] Batch and expiry data model.
-   [ ] Validation and duplicate detection.
-   [ ] Search, keyboard navigation, and form-level error display.
-   [ ] Database/service tests for master records.

**Phase status:** Not confirmed as implemented. Recommended first
functional milestone: Company Settings, followed by
customer/supplier/product masters.

### Phase 5 --- Purchase and Inventory

-   [ ] Purchase entry and supplier invoice details.
-   [ ] Purchase line items, free quantities, discounts, and taxes.
-   [ ] Batch number, expiry date, quantity, and purchase cost tracking.
-   [ ] Stock movement ledger.
-   [ ] Purchase posting in one SQLite transaction.
-   [ ] Purchase return workflow.
-   [ ] Stock adjustment and stock transfer.
-   [ ] Low-stock and expiry detection.
-   [ ] Inventory consistency and rollback tests.

**Phase status:** Planned.

### Phase 6 --- Sales and GST

-   [ ] Sales invoice form and product/batch selection.
-   [ ] Customer selection and credit rules.
-   [ ] Quantity, unit/pack, rate, discount, and GST calculations.
-   [ ] Applicable CGST/SGST versus IGST handling.
-   [ ] Exact monetary arithmetic; do not use `f64` for authoritative
    money calculations.
-   [ ] Invoice-number generation with concurrency-safe uniqueness.
-   [ ] Stock availability and batch selection rules.
-   [ ] Invoice, stock movement, and ledger posting atomically.
-   [ ] Saved invoice snapshots preserve product, price, and tax
    details.
-   [ ] Sales return and credit/debit note workflows as required.
-   [ ] Automated tax, rounding, stock and rollback tests.

**Phase status:** Planned.

### Phase 7 --- Printing and PDF

-   [ ] Choose and test a Rust-compatible PDF library.
-   [ ] Invoice layout and company details.
-   [ ] Printer configuration and paper-size support.
-   [ ] Print preview and PDF export.
-   [ ] Test multi-page invoices, long product names, and totals.
-   [ ] Verify output against the business's invoice requirements.

**Phase status:** Planned.

### Phase 8 --- Accounts and Ledgers

-   [ ] Customer receipts.
-   [ ] Supplier payments.
-   [ ] Customer and supplier ledgers.
-   [ ] Opening balances.
-   [ ] Outstanding balances and settlement rules.
-   [ ] Reversal/correction strategy for posted transactions.
-   [ ] Ensure accounts and source-document posting remain consistent.

**Phase status:** Planned.

### Phase 9 --- Reports

-   [ ] Sales and purchase summaries.
-   [ ] Product-wise sales.
-   [ ] Stock statement and movement history.
-   [ ] Batch and expiry reports.
-   [ ] Customer/supplier ledger and outstanding reports.
-   [ ] Receipt/payment reports.
-   [ ] GST/HSN summaries.
-   [ ] Date and financial-year filters.
-   [ ] Export/print support and reconciliation tests.

**Phase status:** Planned.

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

1.  [x] Replace `src/database.rs` with the database foundation
    implementation.
2.  [x] Run `cargo fmt`, `cargo check`, and `cargo test` successfully
    (user report).
3.  [x] Confirm sign-in still works after the database change (user
    report).
4.  [x] Inspect `%LOCALAPPDATA%\Aster` and confirm the global database,
    three separate financial-year databases, and log file exist (user
    report).
5.  [ ] Close and reopen ASTER; confirm it opens existing databases
    without losing the selected financial year or user records.
6.  [ ] Review `git diff` and commit the database foundation plus this
    progress update together.
7.  [x] Prepare the additive financial-year schema v2 patch: company
    settings storage, customers, suppliers, products/medicines,
    manufacturers, units, indexes, and constraints. (Patch prepared; not
    yet applied to the user's working tree.)
8.  [ ] User applies the schema patch and verifies `cargo fmt`,
    `cargo check`, `cargo test`, and sign-in.
9.  [ ] Implement the Company Settings UI against the new persistence
    API; verify save/reload across restart and financial-year isolation.
10. [ ] Implement customer, supplier, and product CRUD screens after
    Company Settings is verified.

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

**Current overall assessment:** The initial native app and three major
UI screens are visibly implemented. The SQLite foundation has been
replaced and user-reported checks (`cargo fmt`, `cargo check`,
`cargo test`, sign-in) passed. Runtime inspection confirmed the global
DB, separate `2026-2027`, `2027-2028`, and `2028-2029` year DBs, and a
log file under `%LOCALAPPDATA%\Aster`. An additive year-schema v2 patch
has been prepared with company settings and master tables; its SQL
migration was validated using SQLite, but the Rust patch has not yet
been applied or checked by Cargo in the user's Windows project. The
Company Settings UI and real CRUD workflows remain unfinished. The
final-release requirement to keep data inside the ASTER folder is
documented in Phase 10, but is not implemented yet; current development
paths remain unchanged by design.

------------------------------------------------------------------------

## Latest Update --- 2026-10-21

### Company Settings

-   [x] `src/screens/company_settings.rs` implemented.
-   [x] Loads settings for the selected financial year through
    `Database::company_settings(year)`.
-   [x] Saves legal/trade name, GSTIN, drug licences, address/contact,
    invoice prefix and bank details through
    `Database::save_company_settings(year, ...)`.
-   [x] User reported the screen was working flawlessly.
-   [ ] Verify persistence after restarting and switching financial
    years.

### Product Master --- Database

-   [x] Confirmed existing `products`, `manufacturers`, and `units`
    tables in the year-specific SQLite schema.
-   [x] Product schema includes name, brand, generic name, composition,
    manufacturer, unit, HSN, GST, schedule, barcode, reorder level,
    active status and timestamps.
-   [x] Added `ProductRecord`, `ManufacturerOption`, and `UnitOption` in
    `src/database.rs`.
-   [x] Added `Database::products(year, search)` for filtered listing by
    name, brand, generic name, composition and barcode, capped at 500
    results.
-   [x] Added `Database::manufacturers(year)` and
    `Database::units(year)` for active dropdown options.
-   [x] Added `Database::save_product(year, id, product)` for
    insert/update and validation of name, GST range and reorder level;
    duplicate-key errors are handled.
-   [x] Added `Database::set_product_active(year, id, active)`.
-   [x] User reported `cargo check` produced warnings only after adding
    the database methods.
-   [ ] Review/clean warnings after the UI is finalized.
-   [ ] Test inserts, updates, duplicate handling, search and
    activation/deactivation against the selected year's database.

### Product Master --- UI

-   [x] Registered `product_master` in `src/screens/mod.rs`.
-   [x] Routed `WorkspacePage::Products` to
    `super::product_master::show(ui, self)` in
    `src/screens/workspace.rs`.
-   [x] Created `src/screens/product_master.rs` with search, New
    Product/Clear Form actions, product fields, manufacturer/unit
    selectors, product table, Edit and Activate/Deactivate actions.
-   [x] Screenshot confirms the Products / Medicines page opens and
    renders the table with `0 product(s) shown`.
-   \[\~\] UI currently appears light/default-styled in the screenshot;
    visual consistency with ASTER's intended dark-first design remains
    unfinished.
-   [ ] Confirm intended theme before changing theme styling.
-   [ ] Test create/save, edit/reload, search by
    name/brand/composition/barcode, and activation/deactivation.
-   [ ] Confirm table refreshes immediately after save/status changes
    and error feedback is visible.
-   [ ] Improve layout sizing, table readability and visual consistency.

### Current Navigation

-   [x] `WorkspacePage::Products` already existed in the navigation enum
    and sidebar as "Products / Medicines".
-   [x] Product Master now routes to a functional screen instead of the
    generic placeholder.
-   \[\~\] Other workspace routes may still be placeholders and need
    verification.

### Immediate Next Steps

1.  Run `cargo fmt`, `cargo check`, and `cargo run` on the current code.
2.  Test product creation, editing, searching, and
    activation/deactivation with a test product.
3.  Resolve warnings and functional defects.
4.  Refine Product Master styling after confirming the desired theme.
5.  Update this progress document after those checks are completed.

**Accuracy note:** A screen rendering confirms routing and UI display,
not complete persistence correctness. Product CRUD and status toggling
still require explicit runtime testing.
