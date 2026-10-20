# ASTER — Development Progress

> **Project:** ASTER Medical Wholesale Billing ERP  
> **Stack:** Rust + `egui`/`eframe` + SQLite (`rusqlite`)  
> **Last updated:** 2026-10-19  
> **Purpose:** Track completed work, current status, planned phases, and module responsibilities.  
>
> **Status convention:** `[x]` confirmed by project details/screenshots; `[~]` partially implemented or visible but not yet verified; `[ ]` planned/not yet confirmed.  
> This document reflects the progress reported so far. A checkbox does not claim hidden persistence or business logic works unless it has been tested.

---

## 1. Project Principles

- [x] Build a fresh ASTER application rather than importing legacy MDB records.
- [x] Use Rust and native `egui`/`eframe` for a lightweight Windows desktop application.
- [x] Use SQLite through `rusqlite` with the `bundled` feature; no separate SQLite server is required.
- [x] Keep the current native OS window decorations.
- [x] Use Git and the remote `origin/main` for source history.
- [ ] Confirm automated backup and database restore procedures.
- [ ] Confirm all business calculations and database operations with automated tests.

## 2. Environment and Toolchain

- [x] Rust toolchain installed.
- [x] GNU Rust toolchain installed: `stable-x86_64-pc-windows-gnu`.
- [x] GNU toolchain set as default, according to user report.
- [x] `eframe`/`egui` dependency compilation check succeeded with the GNU toolchain.
- [x] Native desktop UI launched successfully with the GNU toolchain, according to user report.
- [x] Git repository connected to `origin/main`; the reported branch was up to date with the remote.
- [x] Visual Studio Build Tools avoided due to limited disk space.
- [ ] Confirm a clean `cargo run` on the current project state.
- [ ] Confirm `cargo fmt`, `cargo check`, `cargo test`, and `cargo clippy` pass on the current project state.
- [ ] Document exact Rust toolchain and build commands in the project README/DEVELOPMENT guide.

## 3. Dependencies Confirmed in `aster/Cargo.toml`

- [x] `eframe = "0.33"` — native application framework.
- [x] `egui = "0.33"` — immediate-mode UI.
- [x] `image = "0.25.10"` with PNG support — image asset loading.
- [x] `argon2 = "0.5"` — password hashing dependency.
- [x] `password-hash = "0.5"` with `getrandom` — password-hash support.
- [x] `rusqlite = "0.38"` with `bundled` — SQLite integration and bundled SQLite library.
- [x] `keyring = "3"` — OS credential-store integration dependency.
- [ ] Verify each dependency is actually used correctly in runtime code.
- [ ] Confirm `Cargo.lock` is committed after reviewing the staged changes.

## 4. Application Flow and UI

### 4.1 Date and Financial-Year Selection

Module: `src/screens/year_date.rs`; supporting date module: `src/date.rs`.

- [x] A working-period selection screen is visible.
- [x] Financial years are displayed as selectable options.
- [x] Current financial year shown in the screenshot: `2026–2027`.
- [x] Working date is displayed in `DD/MM/YYYY` format in the screenshot.
- [x] Calendar navigation and date selection UI are visible.
- [x] A “Today” control is visible.
- [x] UI states the financial year runs from 1 April to 31 March.
- [x] Continue action leads to the sign-in screen, as shown.
- [ ] Verify date input validation and boundary handling in code.
- [ ] Verify a selected date must fall within the selected financial year.
- [ ] Verify financial-year selection is saved and restored appropriately.
- [ ] Verify adding the next financial year does not create duplicates or invalid ranges.

### 4.2 Sign-In Screen

Module: `src/screens/login.rs`.

- [x] A sign-in screen is visible.
- [x] Username field is displayed.
- [x] Password field is displayed with a visibility-toggle control.
- [x] “Remember my password on this PC” option is visible.
- [x] Sign-in and Back controls are visible.
- [x] The screen displays the selected financial year and working date.
- [x] `argon2`, `password-hash`, and `keyring` dependencies are declared.
- [ ] Verify credential validation is connected to persistent user records.
- [ ] Verify password hashes are used rather than plaintext password storage.
- [ ] Verify “remember password” uses Windows Credential Manager/keyring correctly and only after user consent.
- [ ] Verify incorrect credentials, empty fields, locked accounts, and sign-out behaviour.
- [ ] Verify first-run administrator creation and recovery process.

### 4.3 Main Workspace and Dashboard

Module: `src/screens/workspace.rs`.

- [x] Main workspace/dashboard is visible.
- [x] ASTER branding and user/profile area are visible.
- [x] Top tab strip is visible with Dashboard selected.
- [x] Sidebar navigation is visible.
- [x] Sidebar sections shown: Workspace, Masters, Transactions, Accounts & Reports, System.
- [x] Navigation entries shown: Overview, Company settings, Customers, Suppliers, Products/Medicines, Purchase, Sales invoice, Stock, Accounts, Reports, Settings.
- [x] Dashboard summary cards are visible: Today's sales, Stock alerts, Receivables, Expiring batches.
- [x] Recent activity panel is visible.
- [x] Quick actions are visible: New sales invoice, Record purchase, Add product, Check stock, Open reports.
- [x] Status bar displays “Database connected” and the selected financial year/date in the screenshot.
- [ ] Verify navigation entries open functional screens rather than placeholders.
- [ ] Verify tab switching, closing tabs, duplicate-tab prevention, and unsaved-change warnings.
- [ ] Verify dashboard figures are database-driven rather than placeholders.
- [ ] Verify the “Database connected” status is based on an actual successful connection and query.

## 5. Source Modules Currently Reported

The following files were listed in the current project:

| Module/file | Current status | Known responsibility / next verification |
|---|---|---|
| `src/main.rs` | Present | Application entry point; inspect startup and error handling. |
| `src/app.rs` | Present | Application state and top-level routing; inspect lifecycle and screen state. |
| `src/database.rs` | Present | Database access; inspect actual path, schema, migrations, and connection handling. |
| `src/date.rs` | Present, modified/staged | Date and financial-year logic; inspect validation and tests. |
| `src/logging.rs` | Present | Logging setup; verify file paths, log rotation, and error reporting. |
| `src/theme.rs` | Present | Shared UI theme; verify consistency and accessibility. |
| `src/screens/mod.rs` | Present | Screen module declarations. |
| `src/screens/year_date.rs` | Present, modified/staged | Working-date and financial-year screen. |
| `src/screens/login.rs` | Present, modified/staged | Sign-in screen and authentication UI. |
| `src/screens/workspace.rs` | Present, modified/staged | Main workspace, sidebar, dashboard and navigation UI. |

**Important:** The file list confirms that modules exist; it does not by itself confirm their internal implementations or correctness.

## 6. Git / Current Working State

Latest reported `git status` was run from `D:\PROJECT\ASTER`:

- [x] Branch: `main`.
- [x] Local branch reported up to date with `origin/main`.
- [x] These six files were staged for commit at the time of the report:
  - `aster/Cargo.lock`
  - `aster/Cargo.toml`
  - `aster/src/date.rs`
  - `aster/src/screens/login.rs`
  - `aster/src/screens/workspace.rs`
  - `aster/src/screens/year_date.rs`
- [ ] Review staged diff before committing.
- [ ] Commit the current working milestone with a clear message.
- [ ] Create the next feature branch after the current work is committed.
- [ ] Push each reviewed milestone to the remote.

Suggested review commands:

```powershell
git diff --cached --stat
git diff --cached
```

Do not reset or discard staged changes without reviewing them.

## 7. Development Phases

### Phase 0 — Environment and Initial Native Window

- [x] Rust project created.
- [x] `eframe`/`egui` configured.
- [x] GNU toolchain build check passed.
- [x] Native window launched.
- [x] Git remote push completed, according to user report.

**Phase status:** Complete for the initial scaffold.

### Phase 1 — Application Shell and Entry Flow

- [x] Financial-year/date selection UI.
- [x] Sign-in UI.
- [x] Main workspace/dashboard UI.
- [x] Sidebar and navigation categories.
- [x] Dashboard cards and quick actions.
- [x] Native window title bar retained.
- [~] Authentication, date/year selection, and navigation behaviour need code-level verification.
- [ ] Finish tab lifecycle and screen-routing tests.
- [ ] Confirm UI behaviour on different window sizes and scaling settings.

**Phase status:** UI is implemented to the point shown in screenshots; functional completeness is not yet verified.

### Phase 2 — SQLite Foundation

- [x] `rusqlite` dependency declared with `bundled`.
- [~] A database module exists at `src/database.rs`.
- [~] Dashboard screenshot reports “Database connected”; connection details are not yet verified.
- [ ] Inspect and document actual database file path.
- [ ] Implement/verify a single connection policy appropriate for the application.
- [ ] Add versioned schema migrations.
- [ ] Create schema migration tracking.
- [ ] Enable and verify foreign-key enforcement.
- [ ] Configure safe busy timeout and appropriate journal mode.
- [ ] Define transaction boundaries for business operations.
- [ ] Add tests for fresh database initialization, reopening, migration, and rollback.
- [ ] Add safe backup and restore procedures.
- [ ] Confirm no legacy MDB data is imported.

**Phase status:** Dependencies and module are present; schema and persistence need inspection.

### Phase 3 — Company Setup, Financial Year, and Authentication

- [x] Financial-year/date UI exists.
- [x] Sign-in UI exists.
- [x] Authentication-related dependencies are declared.
- [ ] Persist company profile.
- [ ] Define financial-year records and validation rules.
- [ ] Implement secure first-run administrator setup.
- [ ] Persist users and password hashes.
- [ ] Implement role/permission checks.
- [ ] Implement safe remembered-credential behaviour.
- [ ] Add audit events for security-sensitive actions.

**Phase status:** UI foundation exists; data-backed behaviour remains unverified/planned.

### Phase 4 — Master Data

- [ ] Company settings persistence and reload.
- [ ] Customer creation, search, edit, and deactivate.
- [ ] Supplier creation, search, edit, and deactivate.
- [ ] Product/medicine master.
- [ ] Manufacturer, unit/pack configuration, HSN and tax-rate fields.
- [ ] Batch and expiry data model.
- [ ] Validation and duplicate detection.
- [ ] Search, keyboard navigation, and form-level error display.
- [ ] Database/service tests for master records.

**Phase status:** Not confirmed as implemented. Recommended first functional milestone: Company Settings, followed by customer/supplier/product masters.

### Phase 5 — Purchase and Inventory

- [ ] Purchase entry and supplier invoice details.
- [ ] Purchase line items, free quantities, discounts, and taxes.
- [ ] Batch number, expiry date, quantity, and purchase cost tracking.
- [ ] Stock movement ledger.
- [ ] Purchase posting in one SQLite transaction.
- [ ] Purchase return workflow.
- [ ] Stock adjustment and stock transfer.
- [ ] Low-stock and expiry detection.
- [ ] Inventory consistency and rollback tests.

**Phase status:** Planned.

### Phase 6 — Sales and GST

- [ ] Sales invoice form and product/batch selection.
- [ ] Customer selection and credit rules.
- [ ] Quantity, unit/pack, rate, discount, and GST calculations.
- [ ] Applicable CGST/SGST versus IGST handling.
- [ ] Exact monetary arithmetic; do not use `f64` for authoritative money calculations.
- [ ] Invoice-number generation with concurrency-safe uniqueness.
- [ ] Stock availability and batch selection rules.
- [ ] Invoice, stock movement, and ledger posting atomically.
- [ ] Saved invoice snapshots preserve product, price, and tax details.
- [ ] Sales return and credit/debit note workflows as required.
- [ ] Automated tax, rounding, stock and rollback tests.

**Phase status:** Planned.

### Phase 7 — Printing and PDF

- [ ] Choose and test a Rust-compatible PDF library.
- [ ] Invoice layout and company details.
- [ ] Printer configuration and paper-size support.
- [ ] Print preview and PDF export.
- [ ] Test multi-page invoices, long product names, and totals.
- [ ] Verify output against the business's invoice requirements.

**Phase status:** Planned.

### Phase 8 — Accounts and Ledgers

- [ ] Customer receipts.
- [ ] Supplier payments.
- [ ] Customer and supplier ledgers.
- [ ] Opening balances.
- [ ] Outstanding balances and settlement rules.
- [ ] Reversal/correction strategy for posted transactions.
- [ ] Ensure accounts and source-document posting remain consistent.

**Phase status:** Planned.

### Phase 9 — Reports

- [ ] Sales and purchase summaries.
- [ ] Product-wise sales.
- [ ] Stock statement and movement history.
- [ ] Batch and expiry reports.
- [ ] Customer/supplier ledger and outstanding reports.
- [ ] Receipt/payment reports.
- [ ] GST/HSN summaries.
- [ ] Date and financial-year filters.
- [ ] Export/print support and reconciliation tests.

**Phase status:** Planned.

### Phase 10 — Reliability and Release

- [ ] Tested database backup and restore.
- [ ] Startup recovery and clear database-error handling.
- [ ] Audit logging and permissions review.
- [ ] Performance checks with realistic data volumes.
- [ ] Full regression test suite.
- [ ] Installer/package tested on a clean Windows environment.
- [ ] Versioning and upgrade/migration strategy.
- [ ] User documentation and recovery instructions.
- [ ] Final acceptance testing with the business owner.

**Phase status:** Planned.

## 8. Immediate Next Actions

1. Review the staged Git diff and preserve the current UI work.
2. Inspect `src/database.rs` and `src/main.rs`.
3. Verify the actual database path and whether the connection is real.
4. Verify the app can create/open the database and preserve data after restart.
5. Implement or complete schema migrations and tests before adding business records.
6. Build a Company Settings form that saves and reloads real values.
7. Continue to Customers, Suppliers, and Products only after persistence is tested.

## 9. Definition of “Complete”

A screen is not considered complete merely because it renders. Mark a feature complete only when:

- It validates user input.
- It persists data where required.
- It reloads correctly after restarting the application.
- It handles invalid input and database errors safely.
- Its business rules have tests.
- It has no fake success messages or nonfunctional controls.
- Changes are reviewed, tested, and committed.

---

**Current overall assessment:** The initial native app and three major UI screens are visibly implemented. Rust/egui and SQLite dependencies are configured. The internal database schema, durable persistence, and business logic have not yet been inspected in enough detail to claim they are complete.
