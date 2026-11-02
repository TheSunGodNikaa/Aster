# ASTER Development Progress

> **Project status (updated 2026-11-02):** Foundation, business masters, purchase/sales workflows, Stock Overview, Accounts, Payment Register, and Reports are implemented. The user confirmed that the corrected `reports.rs` builds and works correctly. Remaining work is focused on broader regression testing, unfinished workflows, System Settings, and release readiness.

## Phase 1 — Foundation and Launch Flow

- [x] Inspect the existing project and reference assets.
- [x] Set up Rust and Cargo for Windows GNU development and configure the required C compiler toolchain.
- [x] Establish the native `eframe` application window and ASTER title.
- [x] Implement the initial application theme and embedded background asset.
- [x] Add financial-year and working-date selection using the April 1 financial-year boundary.
- [x] Add first-run administrator setup and login screens.
- [x] Hash account passwords with Argon2; avoid hard-coded credentials.
- [x] Open the main workspace maximized after successful login.
- [x] Initialize a per-user SQLite database with a versioned schema.
- [x] Add startup logging under the per-user Local AppData directory.
- [x] Add build and development notes.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo check` after modularizing the UI.
- [x] Launch the application and verify that it runs successfully.

**Acceptance:** Foundation and launch flow implemented; formatting/build checks and runtime launch have been verified.

## Phase 2 — Database and Core Business Masters

- [x] Establish SQLite database access for the application.
- [x] Implement company settings.
- [x] Implement Customer Master and enhance its UI.
- [x] Implement Supplier Master.
- [x] Implement Product Master.
- [ ] Review master-data validation, duplicate handling, and referential integrity across all workflows.
- [ ] Add any remaining database migrations required by future modules.

**Acceptance:** Core business masters are implemented. Cross-module validation and future schema changes remain to be reviewed.

## Phase 3 — Workspace Navigation and Transaction Workflows

- [x] Split the application into maintainable UI modules.
- [x] Add the main workspace and module navigation.
- [x] Implement Purchase Entry.
- [x] Implement Sales Invoicing.
- [ ] Complete end-to-end validation of purchase and sales workflows against real business scenarios.
- [ ] Review transaction editing, cancellation, and error/recovery paths.
- [ ] Verify that stock and customer/supplier balances remain consistent after transactions.

**Acceptance:** Primary purchase and sales entry workflows are present; full workflow and consistency testing remain.

## Phase 4 — Stock Overview and Inventory Visibility

- [x] Connect Stock Overview to the database batch records through `stock_batches`.
- [x] Aggregate batch records into product-level stock rows.
- [x] Calculate stock quantities and stock valuation for the overview.
- [x] Display inventory summary metrics, including product count, low-stock count, and stock valuation.
- [x] Add product search and refresh controls.
- [x] Implement the filters: All, Low Stock, Expiring, Expired, and Out of Stock.
- [x] Add product selection and a batch-level breakdown.
- [x] Display batch expiry information in the detail panel.
- [x] Run `cargo check` after the latest changes; no warnings or errors were reported.
- [x] Launch the application and verify that it runs successfully.
- [ ] Restore or implement column sorting, if still required by the intended Stock Overview design.
- [ ] Validate stock quantities, free quantities, expiry handling, and valuation against representative database records.

**Acceptance:** The current Stock Overview implementation and five filters are complete and runtime-verified. Sorting and deeper data validation remain open.

## Phase 5 — Accounts, Payment Register, and Reporting

- [x] Implement Accounts.
- [x] Implement the invoice/payment register UI.
- [x] Implement Reports screen and report selection.
- [x] Add sales and purchase reporting.
- [x] Add stock reporting and expiry indicators.
- [x] Add receivables reporting with ageing.
- [x] Add payment reporting.
- [x] Add GST/HSN summary reporting.
- [x] Add financial-year-aware date filtering and quick date ranges.
- [x] Add report search and column sorting.
- [x] Add pagination for report rows.
- [x] Add CSV export.
- [x] Refine Reports UI spacing, cards, filters, metrics and table layout.
- [x] User confirmed the corrected `reports.rs` works correctly (2026-11-02).
- [ ] Reconcile report totals against representative source transactions and edge cases.
- [ ] Add print/PDF export if required.

**Acceptance:** Accounts, Payment Register and Reports are implemented. Reports have been confirmed working by the user; reconciliation and additional export/print validation remain open.

## Phase 6 — User Administration and Access Control

- [x] Provide first-run administrator setup and login.
- [x] Store passwords as Argon2 hashes.
- [ ] Add account management and password-change workflows.
- [ ] Implement roles and permission checks for restricted actions.
- [ ] Verify session handling and safe behavior after logout or authentication failure.

**Acceptance:** Initial authentication is implemented; ongoing user administration and authorization are pending.

## Phase 7 — Quality Assurance, Packaging, and Release

- [x] Run Rust formatting and compile checks during development.
- [x] Verify that the application launches and runs after the latest Stock Overview changes.
- [ ] Add repeatable tests for database operations and critical business calculations.
- [ ] Test empty databases, malformed or incomplete records, and database error paths.
- [ ] Run end-to-end regression tests across masters, purchases, sales, and stock.
- [ ] Review logging, error messages, and recovery behavior.
- [ ] Finalize release build settings and Windows packaging.
- [ ] Perform a clean-machine installation and startup test.
- [ ] Document backup, restore, and upgrade procedures for the SQLite database.

**Acceptance:** Development checks and a runtime smoke test have passed; release-level QA and packaging remain.

## Current Source Layout

- `aster/src/main.rs` — application entry point and startup window options.
- `aster/src/app.rs` — application state, launch stages, and window transitions.
- `aster/src/screens/` — launch screens, workspace, and feature screens.
- `aster/src/database.rs` — SQLite initialization and database access.
- `aster/src/date.rs` — date and financial-year helpers.
- `aster/src/theme.rs` — shared UI theme.
- `aster/src/logging.rs` — startup and application logging.
- Feature-specific modules — business masters, purchase/sales workflows, Stock Overview, Accounts, Payment Register, and Reports.

> **Maintenance note:** Update this file when a phase changes state. Mark a task complete only after the implementation exists and its relevant check or manual verification has passed. Keep unresolved work in the appropriate phase rather than removing it from the history.


## Latest Update — 2026-11-02 (Accounts and Reports)

### Completed and confirmed

- [x] Accounts module implemented.
- [x] Invoice/payment register UI implemented.
- [x] Reports module completed and corrected in `src/screens/reports.rs`.
- [x] Report categories cover Sales, Purchases, Stock, Receivables, Payments, and GST/HSN.
- [x] Report filters include financial-year dates, custom date ranges, quick ranges, and search.
- [x] Report tables support column sorting and pagination.
- [x] CSV export implemented.
- [x] Stock expiry indicators and receivables ageing included.
- [x] Reports UI spacing and layout refined.
- [x] User confirmed the corrected Reports implementation is working.

### Still pending

- [ ] Validate report calculations against known transaction fixtures and edge cases.
- [ ] Complete end-to-end consistency checks across purchases, stock, sales, receipts, and reports.
- [ ] Implement System Settings beyond its navigation entry.
- [ ] Complete invoice history, print/PDF, returns and reversal workflows where required.
- [ ] Complete backup/restore, packaging, migration, and release testing.

**Update note:** This section supersedes the earlier Phase 5 statement that Accounts and Reports were not implemented. Earlier historical notes are retained for traceability; the struck-through wording is no longer current.
