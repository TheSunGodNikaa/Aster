# ASTER — Complete Coding Agent Implementation Specification

## 1. Your role

Act as a senior Rust desktop application engineer. Build ASTER, a reliable medical wholesale billing, inventory, and accounts application for Windows.

This is a real business application. Prioritize:

1. Correct billing, inventory, GST, and financial calculations.
2. Reliability and protection against data loss.
3. Fast startup, low memory usage, and responsive UI.
4. Simple keyboard-friendly workflows.
5. A clean, professional desktop interface.
6. Maintainable Rust code with automated tests.

Do not merely generate a prototype or static UI. Implement working features connected to the database.

## 2. Mandatory technology stack

- Language: Rust
- GUI: egui
- Desktop framework: eframe
- Database: SQLite using rusqlite
- Serialization: serde and serde_json where needed
- Monetary calculations: integer paise or a suitable exact decimal representation
- Date handling: chrono
- Password hashing: argon2
- Error handling: thiserror and anyhow where appropriate
- Logging: tracing
- Testing: Rust unit and integration tests
- PDF generation: choose and document a suitable maintained Rust library when implementing invoices
- Windows packaging: a standalone Windows desktop executable and installer when the app is ready

Use compatible, maintained crate versions and verify them against the installed Rust toolchain.

Do not use Electron, Tauri, React, a browser-based UI, a web server, or a JavaScript frontend. Keep the application in Rust.

Use SQLite for persistent data. Keep database operations and business rules separate from GUI code.

## 3. Project location and data policy

Project root:\
C:\Users\TheSunGod\Documents\Project\Aster\

Create a clean project if necessary. Inspect the current directory before changing anything. Do not overwrite useful files without checking them first.

The old application used VB6 and Microsoft Access databases. Files such as adhav.docx, concept_db.txt, database.MDB, and stockmonth.mdb may be used as reference material when accessible.

IMPORTANT:

- Do not import, migrate, or copy legacy MDB records.
- Do not depend on the old application's executable or runtime.
- Build a fresh SQLite database for ASTER.
- Reuse business concepts and requirements, not old database contents.
- Do not fabricate customer, supplier, product, or financial records.
- Start with empty business data and provide optional, clearly labelled demonstration data only if requested.
- Never put the production database inside a temporary build directory.

Store application data in an appropriate Windows per-user application-data directory, not in the source-code directory. Provide a visible way to locate the database and backups.

## 4. Required application workflow

Implement the following launch flow:

### Step 1: Application startup

- Initialize logging.
- Locate the application-data directory.
- Open or create the SQLite database.
- Enable foreign-key enforcement.
- Configure appropriate SQLite journaling and synchronization.
- Apply versioned database migrations.
- Check that the database can be read and written.
- Show a helpful error screen if initialization fails.

Never silently replace a corrupt database with a new empty one.

### Step 2: Financial-year selection

Show a financial-year selection window before login.

Requirements:

- Display existing financial years.
- Allow creation of a new financial year.
- Default to the current Indian financial year when appropriate.
- Validate the selected year.
- Display the selected year clearly.
- Preserve the selected year for the current session.
- Do not silently create a second database file for each year.
- Keep all historical records in SQLite, with financial-year filtering and document numbering.
- Prevent accidental duplicate year creation.

Use April 1 to March 31 as the default financial-year convention. Store dates consistently and document the boundary rules.

### Step 3: Authentication

- Show a login window after year selection.
- Support username and password.
- Hash passwords securely; never store plain-text passwords.
- Create the initial administrator through a secure first-run setup flow.
- Provide a way to change the password.
- Handle incorrect credentials without crashing.
- Enforce account-active status and roles.
- Do not create a hard-coded production password.

### Step 4: Main application

After successful login, open the main ASTER window.

Display:

- Application name and logo area.
- Selected financial year.
- Current user.
- Current date.
- Main navigation sidebar.
- Tab bar for open screens.
- Main working area.
- Status bar and helpful validation messages.

The interface should be dark by default. Keep styling consistent, readable, and restrained.

## 5. Main navigation

Create a persistent sidebar with collapsible sections and icons.

### Masters

- Company Settings
- Customers
- Suppliers
- Products
- Manufacturers
- Agents
- Areas
- States
- Units / Units of Measurement

### Transactions

- Purchase
- Purchase Return
- Delivery Challan
- Sales Invoice
- Sales Return
- Credit Note
- Debit Note
- Stock
- Stock Adjustment
- Stock Modification
- Stock Transfer

### Accounts

- Customer Receipts
- Supplier Payments
- Customer Ledger
- Supplier Ledger
- Outstanding Balances
- Opening Balances

### Reports

- Sales Reports
- Purchase Reports
- Sales Return Reports
- Purchase Return Reports
- Stock Reports
- Stock Statement
- Batch and Expiry Reports
- Low Stock Report
- Customer Ledger Report
- Supplier Ledger Report
- Receipts and Payments
- GST Reports
- HSN Summary

### System Tools

- User Management
- Financial-Year Management
- Printer Settings
- Invoice Settings
- Backup Database
- Restore Database
- Audit / Activity Log
- Application Settings
- Exit

Only show features that are implemented and usable. If a future feature is not implemented, mark it clearly as unavailable rather than pretending it works.

## 6. Desktop UI behaviour

Build reusable egui components instead of creating a separate, inconsistent design for every screen.

### Window layout

- Use a clean title/header area.
- Sidebar on the left.
- Tab strip above the working area.
- Content area in the center.
- Status bar at the bottom.
- Support resizing and sensible minimum dimensions.

Use native window decorations initially. Do not spend time building a custom frameless title bar before core functionality works.

### Tabs

- Opening a navigation item opens its screen in a tab.
- Selecting an existing tab switches to it.
- Closing a tab removes it from the open-tab list.
- Prevent duplicate instances of singleton screens where appropriate.
- Warn before closing a form with unsaved changes.
- Preserve form state when switching between tabs.
- Support keyboard shortcuts for tab navigation.

### Forms and tables

Create reusable components for:

- Text fields.
- Numeric fields.
- Date fields.
- Searchable dropdowns.
- Validation errors.
- Confirmation dialogs.
- Data tables.
- Pagination or efficient row rendering.
- Sorting and filtering.
- Save, Cancel, Edit, Delete/Deactivate, and Clear actions.
- Loading, empty, and error states.

Use clear labels and consistent field order. Avoid unnecessary animations and heavy visual effects.

### Keyboard-first workflow

- Tab and Shift+Tab navigate fields.
- Enter selects or confirms where safe.
- Escape closes a dialog or cancels an uncommitted operation.
- Provide useful shortcuts for saving, searching, creating invoices, and switching tabs.
- Never make Enter accidentally post an invoice when the user is still editing a line.

## 7. Architecture and folder structure

Use a modular structure similar to:

src/\
main.rs\
app.rs\
config.rs\
error.rs

database/\
mod.rs\
connection.rs\
migrations.rs\
repositories/

models/\
mod.rs\
customer.rs\
supplier.rs\
product.rs\
batch.rs\
invoice.rs\
ledger.rs\
financial_year.rs\
user.rs

services/\
mod.rs\
auth_service.rs\
customer_service.rs\
supplier_service.rs\
product_service.rs\
inventory_service.rs\
invoice_service.rs\
gst_service.rs\
ledger_service.rs\
report_service.rs\
backup_service.rs

ui/\
mod.rs\
theme.rs\
navigation.rs\
tabs.rs\
components/\
screens/\
financial_year.rs\
login.rs\
dashboard.rs\
company_settings.rs\
customers.rs\
suppliers.rs\
products.rs\
purchase.rs\
sales.rs\
stock.rs\
accounts.rs\
reports.rs\
settings.rs

printing/\
mod.rs\
invoice_template.rs\
pdf.rs

tests/\
database_tests.rs\
inventory_tests.rs\
invoice_tests.rs\
gst_tests.rs\
ledger_tests.rs\
backup_tests.rs

Adapt the structure where necessary, but maintain clear boundaries.

Responsibilities:

- Models define data structures.
- Repositories execute parameterized SQL queries.
- Services implement business rules and transactions.
- UI renders controls and communicates with services.
- Printing handles invoice layout and PDF output.
- Tests verify behaviour independently of the GUI.

Do not put SQL queries, GST calculations, or stock rules directly inside egui rendering functions.

Avoid excessive abstraction. Keep the implementation straightforward and testable.

## 8. Database design

Treat concept_db.txt as a starting reference, not an unquestionable final schema. Review it for correctness before implementing it.

Use versioned migrations and enforce foreign keys.

Core tables should include:

- app_settings
- users
- financial_years
- states
- areas
- manufacturers
- agents
- customers
- suppliers
- products
- batches
- invoices
- invoice_items
- stock_movements
- ledger_entries
- invoice_counters
- printers
- schema_migrations
- audit_log

Add other tables only when a real feature requires them.

### Database rules

- Use primary keys and appropriate foreign keys.
- Use unique constraints for business identifiers.
- Index frequently searched names, invoice numbers, dates, products, batches, and party references.
- Use parameterized SQL exclusively.
- Use SQLite transactions for multi-step business operations.
- Configure WAL and synchronization settings appropriately.
- Enable foreign-key enforcement on every database connection.
- Use migrations for all schema changes.
- Do not create redundant copies of the same financial or inventory data without a clear reason.
- Generate GST and other summaries from transaction data instead of maintaining manually synchronized duplicate summary tables.

### Money and quantity

- Store monetary values as integer paise wherever practical.
- Never use binary floating-point arithmetic for money.
- Use explicit rounding rules for tax and final invoice totals.
- Store percentages and quantities with documented precision.
- Check whether fractional quantities are required before restricting quantities to integers.
- Keep product, batch, and invoice calculations consistent.

### Historical snapshots

Invoice items must preserve the product name, HSN, pack, tax rate, price, and other legally or financially relevant details as they were when the invoice was posted.

Changes to a customer's GSTIN or a product's name must not silently rewrite historical invoices.

### Stock and audit trail

Every stock-affecting transaction must create an auditable stock movement.

Stock changes must be applied exactly once. Do not independently update the same stock balance in multiple places.

Choose one authoritative stock-update mechanism and test it thoroughly. If database triggers are used, ensure the application does not duplicate their effects.

Maintain sufficient movement history to explain how the current balance was reached.

## 9. Master-data screens

Implement these screens before billing.

### Company Settings

Capture:

- Company name.
- Address.
- City and PIN code.
- Phone, mobile, and email.
- GSTIN.
- Drug licence number.
- Invoice prefix and footer message.
- Expiry-alert threshold.

Validate required fields and preserve changes after restart.

### Customers

Fields should include:

- Code.
- Name.
- Address.
- City and PIN code.
- State.
- Area.
- Phone and mobile.
- Email.
- GSTIN.
- Drug licence number where relevant.
- Credit limit.
- Default discount.
- Active/locked status.

### Suppliers

Include supplier code, name, address, state, contact details, GSTIN, drug licence number where relevant, credit limit, discount, and active status.

### Products

Include:

- Product code.
- Product name.
- Description.
- Pack.
- HSN code.
- Unit of measurement.
- GST percentage.
- Manufacturer.
- Reorder level.
- Active status.

### Other masters

Provide functional CRUD screens for manufacturers, agents, areas, states, and units.

### Master-data acceptance criteria

- Create, edit, search, and list records.
- Validate mandatory fields.
- Detect duplicate codes.
- Prevent deletion of records that are referenced by transactions; offer deactivation instead.
- Display understandable error messages.
- Persist data after restarting ASTER.

## 10. Purchasing and batch inventory

Implement Purchase first because stock must enter the system before sales can be tested.

### Purchase workflow

1. Open Purchase in a tab.
2. Select supplier.
3. Enter supplier invoice number and invoice date.
4. Enter the relevant document details.
5. Add products to the item grid.
6. Select or create the batch.
7. Enter batch number and expiry date.
8. Enter quantity and free quantity.
9. Enter purchase price, MRP, PTR, discount, and applicable tax details.
10. Calculate line totals and invoice totals.
11. Show a preview before posting.
12. Validate the document.
13. Post it using one SQLite transaction.

Posting must:

- Save the invoice header.
- Save all invoice items.
- Update or create the relevant batches.
- Record stock movements.
- Record the supplier ledger effect.
- Advance invoice numbering safely.
- Commit everything together.

If any step fails, roll back the entire transaction.

Do not permit duplicate posting from repeated clicks.

### Batch rules

- Track product, batch number, expiry, prices, and quantities.
- Validate batch uniqueness according to the selected schema.
- Display expired and near-expiry batches.
- Warn when expiry information is missing where required.
- Use FEFO—first expiry, first out—as the default batch suggestion for sales.
- Allow the operator to choose another eligible batch when appropriate.
- Do not allow negative stock by default.
- Do not allow sale quantities to exceed available stock.

### Purchase returns

- Reference the original purchase where applicable.
- Reduce the correct batch quantities.
- Record a negative stock movement.
- Update supplier balances and tax totals correctly.
- Prevent returning more stock than permitted by the original purchase and remaining returnable quantity, unless an authorized exception is explicitly supported.

## 11. Sales and invoicing

Make Sales the primary operational screen after Purchase is reliable.

### Sales form

Include:

- Invoice number generated by the application.
- Invoice date.
- Customer search and selection.
- Customer GSTIN and state.
- Agent, if applicable.
- Product search.
- Batch selection.
- Expiry date.
- Available quantity.
- Quantity and free quantity.
- MRP, PTR, and selling rate.
- Discount.
- HSN and GST percentage.
- Line amount and tax.
- Additional charges and round-off.
- Final net amount.

Use a fast item-entry grid. Product search should work by code and name, and the interface should support keyboard-driven entry.

### Invoice calculations

For each line:

1. Validate quantity, rate, discount, and tax inputs.
2. Calculate the line amount according to documented rules.
3. Calculate the taxable amount.
4. Calculate GST using the applicable rate and rounding policy.
5. Split tax into CGST/SGST for qualifying intra-state supplies.
6. Calculate IGST for qualifying inter-state supplies.
7. Calculate the final total and any permitted round-off.

Determine the intra-state/inter-state treatment using the company's registered state and the customer's place of supply, subject to applicable rules.

Never guess tax treatment when required state information is missing. Display a validation error.

Do not hard-code tax rates into the UI. Use the product's configured rate and support valid overrides only when permitted.

Document and test the exact formula and rounding policy. Validate the resulting GST behaviour with the business owner before production use.

### Posting a sale

Inside one database transaction:

- Validate the invoice and selected batches.
- Confirm stock availability.
- Save the invoice and item snapshots.
- Record stock movements.
- Reduce the relevant batch balances exactly once.
- Create the customer ledger entry.
- Advance the invoice counter.
- Commit the document.

On failure, roll back every effect.

After posting, show the invoice number, final amount, and options to print, save PDF, or start a new invoice.

### Invoice numbering

- Maintain sequential numbering per document type and financial year.
- Generate numbers inside a transaction to avoid duplicates.
- Enforce uniqueness in SQLite.
- Do not reuse a posted invoice number.
- Keep cancelled documents auditable.
- Document the policy for draft, cancelled, and posted invoices.

### Invoice preview and printing

Create a readable wholesale invoice containing:

- Company identity and address.
- GSTIN and relevant licence details.
- Invoice number and date.
- Customer details.
- Item descriptions, HSN, batches, expiry, quantities, rates, discounts, and taxes.
- Tax summary.
- Subtotal, charges, round-off, and final total.
- Amount in words where required.
- Footer terms and signature area.

Implement PDF generation and a practical Windows print workflow. Keep invoice rendering separate from sales calculations.

Verify the layout on actual PDF output before treating printing as complete.

## 12. Other transaction screens

After Purchase and Sales are tested, implement:

### Sales Return

- Link to the original sale where possible.
- Validate returnable quantities.
- Restore stock to the appropriate batch.
- Record ledger and tax effects.
- Keep the return auditable.

### Delivery Challan

- Create a delivery document with customer, items, quantities, and dates.
- Make it clear whether the document is non-financial or later converted into a sale.
- Do not change stock or customer balances twice during conversion.

### Credit Note and Debit Note

- Support financial and commercial note types where applicable.
- Record a reason and reference document.
- Apply the correct ledger and tax effects.
- Do not treat every note as a stock movement.

### Stock Adjustment

- Record the reason and authorized user.
- Show before-and-after quantities.
- Create a movement entry.
- Prevent silent stock edits.

### Stock Modification and Transfer

- Require a clear audit trail.
- Validate source and destination.
- Record both sides of a transfer atomically.
- Do not let the operator change stock balances without a corresponding movement record.

## 13. Accounts and ledgers

Use a unified ledger-entry model for customer and supplier money movements where practical.

### Customer Receipts

- Select customer.
- Enter date, amount, payment method, reference, and remarks.
- Support cheque number/date if required.
- Record the receipt.
- Reduce the customer's outstanding balance.
- Prevent invalid or unintended duplicate postings.

### Supplier Payments

- Select supplier.
- Enter payment details.
- Record the payment and update supplier outstanding balances.

### Ledgers

Display:

- Date.
- Document reference.
- Entry type.
- Debit.
- Credit.
- Running balance.
- Remarks.

Implement customer and supplier outstanding reports.

Define debit/credit semantics consistently and test them with examples. Do not rely on ambiguous signs or reverse balances differently across screens.

Opening balances must be entered through a dedicated workflow and must not be duplicated when the application restarts.

## 14. Reports

Implement reports only against persisted transaction data.

Required reports:

- Daily and date-range sales.
- Purchase and purchase returns.
- Sales returns.
- Product-wise sales.
- Customer-wise sales.
- Supplier-wise purchases.
- Stock on hand.
- Batch-wise stock.
- Expiring and expired batches.
- Low stock.
- Stock movement history.
- Customer and supplier ledgers.
- Customer and supplier outstanding balances.
- Receipts and payments.
- GST summaries.
- HSN-wise summary.

Provide date filters, financial-year filters, useful sorting, and totals.

Generate GST and HSN summaries from the underlying invoices and invoice items. Do not store stale duplicate report tables.

Test report totals against source transactions.

## 15. GST and financial correctness

Treat GST as a critical correctness requirement.

- Preserve the tax rates and values used on posted documents.
- Keep historical invoice data immutable through normal master-data editing.
- Use exact arithmetic and explicit rounding.
- Handle intra-state and inter-state supplies.
- Handle applicable discounts, additional charges, returns, credit notes, and debit notes consistently.
- Keep taxable value, CGST, SGST, IGST, cess if applicable, and final totals reconcilable.
- Do not advertise reports as legally compliant merely because they display GST totals.
- Flag uncertain tax rules for confirmation rather than inventing them.

Create unit tests for calculation edge cases, including fractional rates where allowed, discounts, rounding boundaries, zero-tax products, and returns.

## 16. Backup, restore, and safety

Backups are mandatory before production use.

### Backup

- Provide a visible Backup Database action.
- Let the operator choose a destination.
- Use SQLite's supported backup approach rather than copying a live database file unsafely.
- Include a timestamp in the backup name.
- Verify the backup can be opened.
- Show success or failure clearly.

### Restore

- Require confirmation.
- Validate the selected backup.
- Make a safety backup of the current database first.
- Prevent restoration while active transactions are running.
- Restore atomically where practical.
- Reopen the database and run integrity checks.
- Never silently overwrite the only good database.

### General safety

- Use an audit log for important actions.
- Avoid permanent deletion of financial records.
- Use status changes and cancellation workflows where appropriate.
- Handle crashes and database errors without hiding them.
- Never log passwords or sensitive authentication material.

## 17. Performance requirements

The target is a responsive desktop application with low resource usage.

- Avoid unnecessary background threads and polling.
- Keep GUI frames lightweight.
- Do not execute slow report queries repeatedly during rendering.
- Run potentially expensive database/report operations outside the immediate UI rendering path when necessary.
- Return only the records needed for the current view.
- Add appropriate indexes.
- Use prepared statements where useful.
- Avoid loading the entire database into memory.
- Avoid excessive cloning and unnecessary allocations.
- Keep long-running operations cancellable where practical.
- Use transactions for bulk writes.
- Measure performance instead of claiming it is fast without evidence.

Test startup, navigation, searching, and invoice entry on a modest Windows PC.

## 18. Development phases — follow this order

Do not attempt to implement the entire application in one giant code generation step.

### Phase 1 — Foundation

1. Inspect the project directory and installed toolchain.
2. Verify Rust, Cargo, and required Windows build tools.
3. Create an eframe application.
4. Launch a window with a basic ASTER title.
5. Add logging and error handling.
6. Set up formatting and linting.
7. Initialize Git.
8. Add a README and .gitignore.
9. Build and run the app.

Acceptance: ASTER starts reliably and can be rebuilt from a clean checkout.

### Phase 2 — Database foundation

1. Add rusqlite.
2. Establish the application-data directory.
3. Create database connection management.
4. Enable foreign keys and appropriate journaling.
5. Implement versioned migrations.
6. Add core tables and indexes.
7. Add repository patterns and error types.
8. Write database tests.
9. Test restart persistence and failed transactions.

Acceptance: a fresh database initializes correctly, data survives restart, and failed writes roll back.

### Phase 3 — Application shell

1. Implement the theme.
2. Create the main layout.
3. Add sidebar sections.
4. Add tabs.
5. Add status bar.
6. Add reusable forms, dialogs, tables, and search components.
7. Implement navigation and keyboard shortcuts.
8. Add unsaved-changes warnings.

Acceptance: navigation and tabs work without placeholder buttons pretending to perform business operations.

### Phase 4 — Financial year and login

1. Implement first-run administrator creation.
2. Implement financial-year selection.
3. Implement secure login.
4. Add session state and logout.
5. Add role checks.
6. Test invalid credentials and inactive users.

Acceptance: the complete launch workflow works with a fresh installation.

### Phase 5 — Master data

1. Company settings.
2. States and areas.
3. Manufacturers and agents.
4. Units.
5. Customers.
6. Suppliers.
7. Products.
8. Search, validation, duplicate detection, and deactivation.
9. Tests for each repository and service.

Acceptance: all master records persist and can be searched and edited safely.

### Phase 6 — Inventory and purchasing

1. Batch management.
2. Stock movement service.
3. Purchase entry.
4. Purchase posting transaction.
5. Purchase returns.
6. Stock adjustment.
7. Batch expiry and low-stock queries.
8. Automated tests for all stock changes.

Acceptance: purchasing increases stock exactly once; returns and adjustments update stock correctly.

### Phase 7 — Sales

1. Invoice numbering.
2. Customer and product lookup.
3. Fast item-entry grid.
4. Batch selection and availability.
5. GST calculation service.
6. Invoice preview.
7. Atomic invoice posting.
8. Customer ledger integration.
9. PDF invoice generation.
10. Sales tests.

Acceptance: a sale can be entered, posted, reopened, printed, and reconciled with stock and ledger records.

### Phase 8 — Remaining transactions

1. Sales returns.
2. Delivery challans.
3. Credit notes.
4. Debit notes.
5. Stock transfers.
6. Tests for all financial and inventory effects.

Acceptance: each document has correct, auditable effects and cannot accidentally post twice.

### Phase 9 — Accounts and reports

1. Customer receipts.
2. Supplier payments.
3. Opening balances.
4. Ledgers.
5. Outstanding balances.
6. Sales and purchase reports.
7. Stock and expiry reports.
8. GST and HSN reports.
9. Reconciliation tests.

Acceptance: report totals agree with posted transactions and ledger balances.

### Phase 10 — Production readiness

1. Backup and restore.
2. Audit logs.
3. Permission checks.
4. PDF layout and Windows printing.
5. Database integrity tests.
6. Error recovery tests.
7. Performance profiling.
8. Windows release build.
9. Installer creation.
10. Test installation, upgrade, backup, and restoration on a clean Windows environment.

Acceptance: the application can be installed, used, backed up, restored, and upgraded without losing business data.

## 19. Testing requirements

Every important feature must have automated tests.

Minimum scenarios:

- Duplicate customer and product codes are rejected.
- A purchase adds the correct stock.
- A sale reduces the correct batch stock.
- Insufficient stock prevents posting.
- A failed invoice transaction leaves no partial invoice, stock, or ledger changes.
- Repeated submission does not duplicate a posting.
- GST splits correctly according to the configured place-of-supply rules.
- Invoice totals reconcile with line totals and taxes.
- Invoice numbers are unique per document type and financial year.
- Returns respect original quantities and restore/reduce stock correctly.
- Receipts and payments change balances correctly.
- Historical invoice snapshots remain unchanged when master data changes.
- Backup and restore preserve database contents.
- Reports reconcile with source records.
- Expired and near-expiry batches are reported correctly.

Use temporary test databases. Tests must not modify the real user database.

## 20. Coding-agent operating rules

Follow these rules throughout development:

1. Inspect existing files before creating or replacing them.
2. Work in the project root.
3. Implement one phase at a time.
4. Before editing, state the goal and files that need to change.
5. Make focused changes rather than rewriting working modules.
6. After each meaningful change, run cargo fmt --check, cargo check, relevant tests, and clippy when appropriate.
7. Fix compiler errors and test failures before proceeding.
8. Never suppress errors just to obtain a successful build.
9. Do not leave TODOs or fake success messages in completed features.
10. Do not create nonfunctional buttons.
11. Do not add dependencies without a reason.
12. Do not invent tax rules, business policies, or required fields when uncertain; document the question for confirmation.
13. Keep a progress checklist in DEVELOPMENT.md.
14. Update README.md with setup, run, test, and backup instructions.
15. Make a Git commit after each verified milestone.
16. Never delete the database or reset migrations as a shortcut.
17. Never claim a feature works unless it has been built and tested.
18. If blocked, report the exact error, likely cause, and smallest next action.

## 21. Final delivery requirements

Deliver:

- Complete Rust source code.
- SQLite migrations.
- Unit and integration tests.
- README with setup instructions.
- DEVELOPMENT.md with phase completion status.
- Clear database and backup locations.
- A working debug build.
- A working optimized release build.
- PDF invoice output.
- A practical Windows print workflow.
- An installer when packaging is completed.

At the end of every phase, report:

- What was implemented.
- Files created or modified.
- Commands executed.
- Build and test results.
- Known issues.
- Next phase.