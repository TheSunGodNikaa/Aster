\# ASTER — App structure and workflow



ASTER will be a Windows desktop medical wholesale billing and inventory application, built with Rust + egui + SQLite. The goal is to make everyday billing fast, keep stock and accounts accurate, and make the app easy to navigate without unnecessary complexity.



\## 1. Overall application flow



Launch ASTER



Initialize app and open the database



Select Financial Year



Example: 2026–2027



Login



Username, password and permissions



ASTER Main Workspace



Sidebar · Tabs · Dashboard · Status bar



Masters



Products and parties



Transactions



Purchase and sales



Accounts



Receipts and payments



Reports



Stock, GST and ledgers



Every module uses the same SQLite database and shared business rules.



\## 2. Main window layout



The main screen should feel like a modern Windows desktop application, not a website. Keep the interface compact so staff can work quickly.



\### ASTER



Medical Wholesale ERP



2026–2027



Dashboard



Admin



WORKSPACE



Dashboard



Masters



Transactions



Accounts



Reports



System Tools



Dashboard



Sales Invoice



\## Good morning



Your business overview



Today's Sales



\### ₹—



Stock Alerts



\### —



Receivables



\### ₹—



Expiring Batches



\### —



Quick actions



\\+ New Sale



\\+ Purchase



\\+ Receipt



Ready · Database connected · User: Admin



Conceptual layout mockup, not a running application. Dashboard figures should come from the database, never hard-coded sample values.



\### How the interface behaves



\- Left sidebar: collapsible navigation with expandable menu groups.

\- Top tab bar: each opened screen gets a tab; switch between Sales, Products, Purchase, and Reports without losing unsaved form state.

\- Main content: the selected screen fills the remaining space.

\- Window controls: use standard Windows controls initially.

\- Bottom status bar: shows the selected financial year, current user, database status, and useful messages.

\- Dark theme: default. Keep colors, spacing, tables, buttons, and forms consistent across all modules.



\## 3. Complete navigation tree



This is the proposed application menu. Build the structure in full, but implement modules in phases rather than making every menu item appear functional on day one.



Dashboard



Business summary, quick actions, recent invoices, stock alerts, expiry alerts and outstanding balances.



Masters



\- Company Settings

\- Customers

\- Suppliers

\- Products / Medicines

\- Manufacturers

\- Agents

\- Areas

\- States

\- Units



Transactions



\- Purchase

\- Purchase Return

\- Delivery Challan

\- Sales Invoice

\- Sales Return

\- Credit Note

\- Debit Note

\- Stock

\- Stock Adjustment

\- Stock Modification

\- Stock Transfer



Accounts



\- Customer Receipts

\- Supplier Payments

\- Customer Ledger

\- Supplier Ledger

\- Outstanding Balances

\- Opening Balances



Reports



\- Sales and Purchase

\- Returns

\- Product-wise Sales

\- Stock Statement

\- Batch and Expiry

\- Low Stock

\- Stock Movement

\- Customer and Supplier Ledger

\- Outstanding

\- Receipts and Payments

\- GST and HSN Summary



System Tools



\- Users and Permissions

\- Financial-Year Management

\- Invoice and Printer Settings

\- Backup and Restore

\- Audit Log

\- Application Settings

\- Logout and Exit



\## 4. Detailed workflow of each important module



\### A. Masters — create the data used everywhere



Open Customers / Suppliers / Products



Search existing records or create a new record



Validate required fields and duplicates



Save to SQLite



The record becomes available to other modules.



Examples:



\- Create a supplier once; select that supplier for future purchases.

\- Create a medicine once; use it in purchase and sales invoices.

\- Store the product's HSN, GST rate, unit, and manufacturer.

\- Track each physical batch separately, including expiry and available quantity.



Master records used in posted transactions should generally be deactivated rather than permanently deleted.



\### B. Purchase — stock enters the business



\### 1



Select supplier



\### 2



Enter supplier invoice details



\### 3



Add medicines and batch details



\### 4



Enter quantities, free units, prices, discounts and tax



\### 5



Review totals and validate



\### 6



Post purchase transaction



Successful posting



Stock increases



Supplier balance updates



Movement recorded



All effects must succeed together in one database transaction. If a step fails, nothing should be partially posted.



\### C. Sales — the primary billing workflow



1\\. Start invoice



Generate a unique invoice number and select customer.



2\\. Add medicines



Search by name/code; select eligible batch, expiry and quantity.



3\\. Calculate invoice



Rates, discounts, taxable value, GST and final amount.



4\\. Validate



Check available stock, customer details and tax treatment.



5\\. Post invoice



Save invoice, reduce stock, and create the customer ledger entry atomically.



6\\. Print or save PDF



Preview invoice, print, or create a PDF copy.



Important design choice: use a keyboard-friendly item grid. Staff should be able to search for a medicine, select its batch, enter quantity, and move to the next line without repeatedly using the mouse.



\### D. Accounts — track money received and paid



Customer receipt flow:



1\. Select customer.

2\. Show outstanding balance.

3\. Enter amount, date, payment method and reference.

4\. Validate the payment.

5\. Save receipt and update the ledger.

6\. Refresh outstanding balance.



Supplier payment follows the same pattern but records money paid to the supplier.



The ledger must be derived from persisted financial entries. A receipt or payment should never be allowed to update the balance twice.



\### E. Reports — read from real transactions



The user selects a report and filters, such as date range, customer, supplier, product, or financial year.



The report then queries the database, displays rows and totals, and offers suitable export or print options.



\- Stock report → batch balances and stock movements.

\- Sales report → posted sales invoices and returns.

\- Outstanding report → posted invoices, credit/debit adjustments and receipts/payments.

\- GST report → posted invoice tax details and applicable adjustments.

\- Expiry report → batch expiry dates and available quantities.



Reports should not maintain independent copies of totals that can become inconsistent with the original transactions.



\## 5. How the database connects the modules



This is the core relationship the coding agent must preserve.



Master Data



Products · Customers · Suppliers · Units



Posted Documents



Purchases · Sales · Returns · Adjustments



Inventory



Batches and stock movements



Accounts



Ledger entries and balances



Reports and Invoice History



Read from persisted records



For example, posting one sale must update the invoice history, stock, and customer ledger together. If any update fails, the entire posting must roll back.



\## 6. Technical structure inside the app



Keep the Rust code divided into four clear layers.



UI layer — egui



Windows, navigation, tabs, forms, tables, dialogs



Service layer



Billing rules, GST, inventory, authentication and validation



Repository layer



SQL queries, inserts, updates and data retrieval



SQLite



Persistent business records and transaction history



A screen should request an operation from a service; the service validates the request and coordinates database work. The UI should not calculate the authoritative invoice total or directly edit stock balances.



\## 7. Recommended build sequence



| Stage | Build                                  | Completion test                                 |

| ----- | -------------------------------------- | ----------------------------------------------- |

| 1     | App window, theme, navigation and tabs | Open and switch screens reliably                |

| 2     | SQLite and migrations                  | Data survives restart                           |

| 3     | Financial year and login               | Secure launch flow works                        |

| 4     | Masters                                | Create, edit, search and deactivate records     |

| 5     | Purchase and batches                   | Stock increases correctly                       |

| 6     | Sales and GST                          | Invoice posts and stock decreases correctly     |

| 7     | PDF and printing                       | Printed invoice matches stored totals           |

| 8     | Returns and adjustments                | Reversals are accurate and auditable            |

| 9     | Accounts                               | Receipts, payments and balances reconcile       |

| 10    | Reports and backups                    | Reports reconcile; backup restores successfully |



The most important principle: build one complete business workflow at a time. A finished Purchase workflow should actually create batches and stock movements before the agent starts implementing ten more screens.

