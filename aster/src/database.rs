use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, backup::Backup, params};

use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::date::local_date;

const CURRENT_GLOBAL_SCHEMA: i64 = 2;
const CURRENT_YEAR_SCHEMA: i64 = 10;
const MIN_FINANCIAL_YEAR: i32 = 1900;
const MAX_FINANCIAL_YEAR: i32 = 9998;

/// Single source of truth for Indian GST state / jurisdiction codes.
/// Persist the code; use the name only for display.
pub(crate) const GST_STATES: &[(&str, &str)] = &[
    ("01", "Jammu & Kashmir"),
    ("02", "Himachal Pradesh"),
    ("03", "Punjab"),
    ("04", "Chandigarh"),
    ("05", "Uttarakhand"),
    ("06", "Haryana"),
    ("07", "Delhi"),
    ("08", "Rajasthan"),
    ("09", "Uttar Pradesh"),
    ("10", "Bihar"),
    ("11", "Sikkim"),
    ("12", "Arunachal Pradesh"),
    ("13", "Nagaland"),
    ("14", "Manipur"),
    ("15", "Mizoram"),
    ("16", "Tripura"),
    ("17", "Meghalaya"),
    ("18", "Assam"),
    ("19", "West Bengal"),
    ("20", "Jharkhand"),
    ("21", "Odisha"),
    ("22", "Chhattisgarh"),
    ("23", "Madhya Pradesh"),
    ("24", "Gujarat"),
    ("26", "Dadra & Nagar Haveli and Daman & Diu"),
    ("27", "Maharashtra"),
    ("29", "Karnataka"),
    ("30", "Goa"),
    ("31", "Lakshadweep"),
    ("32", "Kerala"),
    ("33", "Tamil Nadu"),
    ("34", "Puducherry"),
    ("35", "Andaman & Nicobar Islands"),
    ("36", "Telangana"),
    ("37", "Andhra Pradesh"),
    ("38", "Ladakh"),
    ("97", "Other Territory"),
    ("99", "Centre Jurisdiction"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GstType {
    IntraState,
    InterState,
}

pub(crate) fn state_name(code: &str) -> Option<&'static str> {
    GST_STATES
        .iter()
        .find_map(|(known_code, name)| (*known_code == code.trim()).then_some(*name))
}

pub(crate) fn is_valid_state_code(code: &str) -> bool {
    state_name(code).is_some()
}

pub(crate) fn gst_type_for_states(
    company_state_code: &str,
    counterparty_state_code: &str,
) -> Result<GstType, String> {
    let company_code = company_state_code.trim();
    let counterparty_code = counterparty_state_code.trim();
    if !is_valid_state_code(company_code) {
        return Err("Set a valid company GST state in Company Settings before posting.".into());
    }
    if !is_valid_state_code(counterparty_code) {
        return Err("Select a valid state for the customer or supplier before posting.".into());
    }
    Ok(if company_code == counterparty_code {
        GstType::IntraState
    } else {
        GstType::InterState
    })
}

pub(crate) fn validate_gstin_state(
    gstin: &str,
    state_code: &str,
    label: &str,
    required: bool,
) -> Result<(), String> {
    let gstin = gstin.trim().to_ascii_uppercase();
    if gstin.is_empty() {
        return if required {
            Err(format!("{label} GSTIN is required."))
        } else {
            Ok(())
        };
    }
    if gstin.len() != 15 || !gstin.is_ascii() {
        return Err(format!("{label} GSTIN must contain 15 characters."));
    }
    let bytes = gstin.as_bytes();
    let valid_shape = bytes[0..2].iter().all(u8::is_ascii_digit)
        && bytes[2..7].iter().all(u8::is_ascii_uppercase)
        && bytes[7..11].iter().all(u8::is_ascii_digit)
        && bytes[11].is_ascii_uppercase()
        && bytes[12].is_ascii_alphanumeric()
        && bytes[13] == b'Z'
        && bytes[14].is_ascii_alphanumeric();
    if !valid_shape {
        return Err(format!("{label} GSTIN format is invalid."));
    }
    let gstin_state = &gstin[..2];
    if !is_valid_state_code(gstin_state) {
        return Err(format!(
            "{label} GSTIN contains an unrecognised state code."
        ));
    }
    if gstin_state != state_code.trim() {
        return Err(format!(
            "{label} GSTIN state code does not match the selected state."
        ));
    }
    Ok(())
}

fn calculate_gst_split(
    taxable_paise: i64,
    gst_basis_points: i64,
    gst_type: GstType,
) -> Result<(i64, i64, i64), &'static str> {
    if taxable_paise < 0 || !(0..=10_000).contains(&gst_basis_points) {
        return Err("GST calculation received an invalid taxable amount or rate.");
    }
    let tax_paise: i64 = ((taxable_paise as i128 * gst_basis_points as i128 + 5_000) / 10_000)
        .try_into()
        .map_err(|_| "GST amount overflow.")?;
    Ok(match gst_type {
        GstType::IntraState => {
            let cgst = tax_paise / 2;
            (cgst, tax_paise - cgst, 0)
        }
        GstType::InterState => (0, 0, tax_paise),
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CompanySettings {
    pub legal_name: String,
    pub trade_name: String,
    pub gstin: String,
    pub drug_license_1: String,
    pub drug_license_2: String,
    pub address_line1: String,
    pub address_line2: String,
    pub city: String,
    pub state: String,
    pub state_code: String,
    pub pincode: String,
    pub phone: String,
    pub email: String,
    pub invoice_prefix: String,
    pub bank_name: String,
    pub bank_account_name: String,
    pub bank_account_number: String,
    pub bank_ifsc: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ProductRecord {
    pub id: i64,
    pub name: String,
    pub brand_name: String,
    pub generic_name: String,
    pub composition: String,
    pub manufacturer_id: Option<i64>,
    pub manufacturer_name: String,
    pub unit_id: Option<i64>,
    pub unit_name: String,
    pub hsn_code: String,
    pub gst_percent: f64,
    pub schedule: String,
    pub barcode: String,
    pub reorder_level: f64,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct StockBatchRecord {
    pub product_id: i64,
    pub product_name: String,
    pub brand_name: String,
    pub unit_name: String,
    pub reorder_level: f64,
    pub batch_id: Option<i64>,
    pub batch_no: String,
    pub expiry_date: String,
    pub quantity_milli: i64,
    pub free_quantity_milli: i64,
    pub purchase_rate_paise: i64,
    pub sale_rate_paise: i64,
    pub mrp_paise: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct CustomerRecord {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub gstin: String,
    pub drug_license: String,
    pub address_line1: String,
    pub address_line2: String,
    pub city: String,
    pub state: String,
    pub state_code: String,
    pub pincode: String,
    pub credit_limit_paise: i64,
    pub opening_balance_paise: i64,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ManufacturerOption {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SupplierRecord {
    pub id: i64,
    pub name: String,
    pub trade_name: String,
    pub contact_person: String,
    pub phone: String,
    pub email: String,
    pub gstin: String,
    pub drug_license: String,
    pub address_line1: String,
    pub address_line2: String,
    pub city: String,
    pub state: String,
    pub state_code: String,
    pub pincode: String,
    pub payment_terms_days: i64,
    pub credit_limit_paise: i64,
    pub opening_balance_paise: i64,
    pub active: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SupplierProductLink {
    pub product_id: i64,
    pub supplier_product_code: String,
    pub preferred: bool,
    pub min_order_qty: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct NewPurchaseProductDraft {
    pub name: String,
    pub brand_name: String,
    pub generic_name: String,
    pub composition: String,
    pub manufacturer_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub hsn_code: String,
    pub gst_percent: f64,
    pub schedule: String,
    pub barcode: String,
    pub reorder_level: f64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PurchaseLineDraft {
    pub product_id: i64,
    pub new_product: Option<NewPurchaseProductDraft>,
    pub batch_no: String,
    pub expiry_date: String,
    pub quantity_milli: i64,
    pub free_quantity_milli: i64,
    pub rate_paise: i64,
    pub discount_basis_points: i64,
    pub mrp_paise: i64,
    pub sale_rate_paise: i64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PurchaseDraft {
    pub supplier_id: i64,
    pub supplier_invoice_no: String,
    pub invoice_date: String,
    pub notes: String,
    pub lines: Vec<PurchaseLineDraft>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SalesLineDraft {
    pub product_id: i64,
    pub quantity_milli: i64,
    pub free_quantity_milli: i64,
    pub rate_paise: i64,
    pub discount_basis_points: i64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SalesInvoiceDraft {
    pub customer_id: i64,
    pub invoice_date: String,
    pub overall_discount_basis_points: i64,
    pub amount_received_paise: i64,
    pub payment_mode: String,
    pub notes: String,
    pub lines: Vec<SalesLineDraft>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SalesInvoiceDetail {
    pub invoice_no: String,

    // Seller: your own company from Company Settings
    pub seller_legal_name: String,
    pub seller_trade_name: String,
    pub seller_gstin: String,
    pub seller_drug_license_1: String,
    pub seller_drug_license_2: String,
    pub seller_address_line1: String,
    pub seller_address_line2: String,
    pub seller_city: String,
    pub seller_state: String,
    pub seller_state_code: String,
    pub seller_pincode: String,
    pub seller_phone: String,
    pub seller_email: String,

    // Buyer: customer from Customer Master
    pub customer_name: String,
    pub customer_gstin: String,
    pub customer_state_code: String,
    pub customer_drug_license: String,
    pub customer_address_line1: String,
    pub customer_address_line2: String,
    pub customer_city: String,
    pub customer_state: String,
    pub customer_pincode: String,
    pub customer_phone: String,

    pub invoice_date: String,
    pub status: String,
    pub taxable_paise: i64,
    pub cgst_paise: i64,
    pub sgst_paise: i64,
    pub igst_paise: i64,
    pub total_paise: i64,
    pub received_paise: i64,
    pub payment_mode: String,
    pub notes: String,
    pub items: Vec<SalesInvoiceItemDetail>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SalesInvoiceItemDetail {
    pub line_no: i64,
    pub product_name: String,
    pub hsn: String,
    pub manufacturer: String,
    pub unit: String,
    pub gst_basis_points: i64,
    pub quantity_milli: i64,
    pub free_quantity_milli: i64,
    pub rate_paise: i64,
    pub discount_basis_points: i64,
    pub taxable_paise: i64,
    pub cgst_paise: i64,
    pub sgst_paise: i64,
    pub igst_paise: i64,
    pub paid_batches: Vec<SalesInvoiceBatchDetail>,
    pub free_batches: Vec<SalesInvoiceBatchDetail>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SalesInvoiceBatchDetail {
    pub quantity_milli: i64,
    pub batch_no: String,
    pub expiry_date: String,
    pub mrp_paise: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct InvoiceDocument {
    pub source: SalesInvoiceDetail,
    pub tax_mode: GstType,
}

impl InvoiceDocument {
    pub(crate) fn from_sales_invoice_detail(source: SalesInvoiceDetail) -> Result<Self, String> {
        let tax_mode = gst_type_for_states(&source.seller_state_code, &source.customer_state_code)
            .map_err(|error| format!("Cannot determine invoice tax mode: {error}"))?;

        let document = Self { source, tax_mode };

        document.validate_consistency()?;

        Ok(document)
    }

    fn checked_line_sum(
        &self,
        value: fn(&SalesInvoiceItemDetail) -> i64,
        label: &str,
    ) -> Result<i64, String> {
        self.source.items.iter().try_fold(0_i64, |total, item| {
            total
                .checked_add(value(item))
                .ok_or_else(|| format!("{label} validation overflow."))
        })
    }

    pub(crate) fn validate_consistency(&self) -> Result<(), String> {
        let taxable = self.checked_line_sum(|item| item.taxable_paise, "Taxable amount")?;

        let cgst = self.checked_line_sum(|item| item.cgst_paise, "CGST")?;

        let sgst = self.checked_line_sum(|item| item.sgst_paise, "SGST")?;

        let igst = self.checked_line_sum(|item| item.igst_paise, "IGST")?;

        if taxable != self.source.taxable_paise {
            return Err(
                "Saved line taxable amounts do not match the invoice taxable total.".into(),
            );
        }

        if cgst != self.source.cgst_paise {
            return Err("Saved line CGST amounts do not match the invoice CGST total.".into());
        }

        if sgst != self.source.sgst_paise {
            return Err("Saved line SGST amounts do not match the invoice SGST total.".into());
        }

        if igst != self.source.igst_paise {
            return Err("Saved line IGST amounts do not match the invoice IGST total.".into());
        }

        match self.tax_mode {
            GstType::IntraState => {
                if self.source.igst_paise != 0 {
                    return Err("Local invoice contains an unexpected IGST amount.".into());
                }
            }
            GstType::InterState => {
                if self.source.cgst_paise != 0 || self.source.sgst_paise != 0 {
                    return Err("Interstate invoice contains unexpected CGST or SGST.".into());
                }
            }
        }

        // The current database does not store a round-off amount.
        // Validate against the exact total produced by invoice posting.
        let expected_total = self
            .source
            .taxable_paise
            .checked_add(self.source.cgst_paise)
            .and_then(|value| value.checked_add(self.source.sgst_paise))
            .and_then(|value| value.checked_add(self.source.igst_paise))
            .ok_or("Invoice total validation overflow.")?;

        if expected_total != self.source.total_paise {
            return Err("Saved invoice total does not match its taxable and tax amounts.".into());
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PurchaseSummary {
    pub id: i64,
    pub purchase_no: String,
    pub supplier_name: String,
    pub supplier_invoice_no: String,
    pub invoice_date: String,
    pub total_paise: i64,
    pub status: String,
}

#[derive(Clone, Debug)]
pub(crate) struct UnitOption {
    pub id: i64,
    pub name: String,
    pub short_name: String,
}

pub(crate) struct Database {
    path: PathBuf,
    data_dir: PathBuf,
}

impl Database {
    pub(crate) fn open() -> Result<Self, String> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| "Windows Local AppData is not available.".to_owned())?;
        Self::open_at(PathBuf::from(local_app_data).join("Aster"))
    }

    fn open_at(directory: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&directory).map_err(|error| format!("Create data folder: {error}"))?;
        let db = Self {
            path: directory.join("aster.sqlite3"),
            data_dir: directory.join("data"),
        };
        let connection = db.connection()?;
        db.migrate_global(&connection)?;

        let (year, month, _) = local_date();
        let current_start = if month >= 4 { year } else { year - 1 };
        connection
            .execute(
                "INSERT OR IGNORE INTO financial_years (start_year) VALUES (?1)",
                [current_start],
            )
            .map_err(|error| format!("Create current financial year: {error}"))?;
        drop(connection);

        // Ensure every registered year has its own empty database, including years
        // that were registered by an earlier version of ASTER.
        for start_year in db.financial_years()? {
            db.initialize_year_database(start_year)?;
        }
        Ok(db)
    }

    fn insert_purchase_product(
        tx: &rusqlite::Transaction<'_>,
        product: &NewPurchaseProductDraft,
        line_no: usize,
    ) -> Result<i64, String> {
        let name = product.name.trim();
        let brand = product.brand_name.trim();
        let composition = product.composition.trim();
        let hsn = product.hsn_code.trim();

        if name.is_empty() {
            return Err(format!("Line {line_no}: product name is required."));
        }

        if hsn.is_empty() {
            return Err(format!("Line {line_no}: HSN code is required."));
        }

        if !product.gst_percent.is_finite() || !(0.0..=100.0).contains(&product.gst_percent) {
            return Err(format!("Line {line_no}: GST must be between 0 and 100."));
        }

        if !product.reorder_level.is_finite() || product.reorder_level < 0.0 {
            return Err(format!("Line {line_no}: reorder level cannot be negative."));
        }

        let manufacturer_id = product
            .manufacturer_id
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("Line {line_no}: select a manufacturer."))?;

        let unit_id = product
            .unit_id
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("Line {line_no}: select a unit."))?;

        // Confirm both selected master records still exist and are active.
        let manufacturer_exists: bool = tx
            .query_row(
                "SELECT EXISTS(
                SELECT 1 FROM manufacturers WHERE id = ?1 AND active = 1
            )",
                [manufacturer_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Line {line_no}: verify manufacturer: {e}"))?;

        if !manufacturer_exists {
            return Err(format!(
                "Line {line_no}: selected manufacturer is unavailable."
            ));
        }

        let unit_exists: bool = tx
            .query_row(
                "SELECT EXISTS(
                SELECT 1 FROM units WHERE id = ?1 AND active = 1
            )",
                [unit_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Line {line_no}: verify unit: {e}"))?;

        if !unit_exists {
            return Err(format!("Line {line_no}: selected unit is unavailable."));
        }

        // Check for an existing product before attempting insertion.
        let duplicate: bool = tx
            .query_row(
                "SELECT EXISTS(
                SELECT 1 FROM products
                WHERE name = ?1 COLLATE NOCASE
                AND brand_name = ?2 COLLATE NOCASE
                AND composition = ?3 COLLATE NOCASE
            )",
                params![name, brand, composition],
                |row| row.get(0),
            )
            .map_err(|e| format!("Line {line_no}: check duplicate product: {e}"))?;

        if duplicate {
            return Err(format!(
                "Line {line_no}: this product already exists. Select the existing product instead."
            ));
        }

        tx.execute(
            "INSERT INTO products (
                name,
                brand_name,
                generic_name,
                composition,
                manufacturer_id,
                unit_id,
                hsn_code,
                gst_percent,
                schedule,
                barcode,
                reorder_level
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                name,
                brand,
                product.generic_name.trim(),
                composition,
                manufacturer_id,
                unit_id,
                hsn,
                product.gst_percent,
                product.schedule.trim(),
                product.barcode.trim(),
                product.reorder_level,
            ],
        ).map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                format!("Line {line_no}: a product with this name, brand and composition already exists.")
            } else {
                format!("Line {line_no}: create product: {e}")
            }
        })?;

        Ok(tx.last_insert_rowid())
    }

    pub(crate) fn connection(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path)
            .map_err(|error| format!("Open database {}: {error}", self.path.display()))?;
        configure_connection(&connection)?;
        Ok(connection)
    }

    fn migrate_global(&self, connection: &Connection) -> Result<(), String> {
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Read global schema version: {error}"))?;
        if version > CURRENT_GLOBAL_SCHEMA {
            return Err(format!(
                "Database schema version {version} is newer than this app."
            ));
        }

        if version < 1 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                    CREATE TABLE financial_years (
                        start_year INTEGER PRIMARY KEY CHECK (start_year BETWEEN 1900 AND 9998),
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    CREATE TABLE users (
                        id INTEGER PRIMARY KEY,
                        username TEXT NOT NULL UNIQUE COLLATE NOCASE,
                        password_hash TEXT NOT NULL,
                        role TEXT NOT NULL DEFAULT 'admin',
                        active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    PRAGMA user_version = 1;
                    COMMIT;",
                )
                .map_err(|error| format!("Apply global schema migration 1: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Read global schema version: {error}"))?;
        if version < 2 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                    CREATE TABLE IF NOT EXISTS schema_migrations (
                        version INTEGER PRIMARY KEY,
                        applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT OR IGNORE INTO schema_migrations (version) VALUES (1);
                    INSERT OR IGNORE INTO schema_migrations (version) VALUES (2);
                    PRAGMA user_version = 2;
                    COMMIT;",
                )
                .map_err(|error| format!("Apply global schema migration 2: {error}"))?;
        }

        let final_version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Verify global schema version: {error}"))?;
        if final_version != CURRENT_GLOBAL_SCHEMA {
            return Err(format!(
                "Global database migration stopped at version {final_version}."
            ));
        }
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|error| format!("Check global database integrity: {error}"))?;
        if integrity != "ok" {
            return Err(format!("SQLite integrity check failed: {integrity}"));
        }
        Ok(())
    }

    fn initialize_year_database(&self, start_year: i32) -> Result<PathBuf, String> {
        validate_financial_year(start_year)?;
        let year_directory = self
            .data_dir
            .join(format!("{start_year}-{}", start_year + 1));
        fs::create_dir_all(&year_directory).map_err(|error| {
            format!(
                "Create data folder for {start_year}–{}: {error}",
                start_year + 1
            )
        })?;
        let path = year_directory.join("aster.sqlite3");
        let connection = Connection::open(&path)
            .map_err(|error| format!("Open financial-year database {}: {error}", path.display()))?;
        configure_connection(&connection)?;

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| {
                format!(
                    "Read {start_year}–{} schema version: {error}",
                    start_year + 1
                )
            })?;
        if version > CURRENT_YEAR_SCHEMA {
            return Err(format!(
                "Financial-year database {start_year}–{} uses newer schema version {version}.",
                start_year + 1
            ));
        }
        if version < 1 {
            let end_year = start_year + 1;
            connection
                .execute_batch(&format!(
                    "BEGIN IMMEDIATE;
                    CREATE TABLE schema_migrations (
                        version INTEGER PRIMARY KEY,
                        applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    CREATE TABLE aster_meta (
                        id INTEGER PRIMARY KEY CHECK (id = 1),
                        financial_year_start INTEGER NOT NULL,
                        financial_year_end INTEGER NOT NULL,
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        CHECK (financial_year_end = financial_year_start + 1)
                    );
                    INSERT INTO aster_meta (id, financial_year_start, financial_year_end)
                        VALUES (1, {start_year}, {end_year});
                    INSERT INTO schema_migrations (version) VALUES (1);
                    PRAGMA user_version = 1;
                    COMMIT;"
                ))
                .map_err(|error| {
                    format!("Initialize database for {start_year}–{end_year}: {error}")
                })?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| {
                format!(
                    "Re-read {start_year}–{} schema version: {error}",
                    start_year + 1
                )
            })?;
        if version < 2 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                    CREATE TABLE company_settings (
                        id INTEGER PRIMARY KEY CHECK (id = 1),
                        legal_name TEXT NOT NULL DEFAULT '',
                        trade_name TEXT NOT NULL DEFAULT '',
                        gstin TEXT NOT NULL DEFAULT '',
                        drug_license_1 TEXT NOT NULL DEFAULT '',
                        drug_license_2 TEXT NOT NULL DEFAULT '',
                        address_line1 TEXT NOT NULL DEFAULT '',
                        address_line2 TEXT NOT NULL DEFAULT '',
                        city TEXT NOT NULL DEFAULT '',
                        state TEXT NOT NULL DEFAULT '',
                        state_code TEXT NOT NULL DEFAULT '',
                        pincode TEXT NOT NULL DEFAULT '',
                        phone TEXT NOT NULL DEFAULT '',
                        email TEXT NOT NULL DEFAULT '',
                        invoice_prefix TEXT NOT NULL DEFAULT 'INV',
                        bank_name TEXT NOT NULL DEFAULT '',
                        bank_account_name TEXT NOT NULL DEFAULT '',
                        bank_account_number TEXT NOT NULL DEFAULT '',
                        bank_ifsc TEXT NOT NULL DEFAULT '',
                        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    CREATE TABLE manufacturers (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                        contact_person TEXT NOT NULL DEFAULT '',
                        phone TEXT NOT NULL DEFAULT '',
                        email TEXT NOT NULL DEFAULT '',
                        address TEXT NOT NULL DEFAULT '',
                        active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    CREATE TABLE units (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                        short_name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                        active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1))
                    );
                    INSERT INTO units (name, short_name) VALUES
                        ('Tablet', 'TAB'), ('Capsule', 'CAP'), ('Bottle', 'BTL'),
                        ('Strip', 'STRIP'), ('Box', 'BOX'), ('Piece', 'PCS'),
                        ('Millilitre', 'ML'), ('Gram', 'GM');
                    CREATE TABLE customers (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL COLLATE NOCASE,
                        phone TEXT NOT NULL DEFAULT '',
                        email TEXT NOT NULL DEFAULT '',
                        gstin TEXT NOT NULL DEFAULT '',
                        drug_license TEXT NOT NULL DEFAULT '',
                        address_line1 TEXT NOT NULL DEFAULT '',
                        address_line2 TEXT NOT NULL DEFAULT '',
                        city TEXT NOT NULL DEFAULT '',
                        state TEXT NOT NULL DEFAULT '',
                        state_code TEXT NOT NULL DEFAULT '',
                        pincode TEXT NOT NULL DEFAULT '',
                        credit_limit_paise INTEGER NOT NULL DEFAULT 0 CHECK (credit_limit_paise >= 0),
                        opening_balance_paise INTEGER NOT NULL DEFAULT 0,
                        active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        UNIQUE(name, phone)
                    );
                    CREATE TABLE suppliers (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL COLLATE NOCASE,
                        phone TEXT NOT NULL DEFAULT '',
                        email TEXT NOT NULL DEFAULT '',
                        gstin TEXT NOT NULL DEFAULT '',
                        drug_license TEXT NOT NULL DEFAULT '',
                        address_line1 TEXT NOT NULL DEFAULT '',
                        address_line2 TEXT NOT NULL DEFAULT '',
                        city TEXT NOT NULL DEFAULT '',
                        state TEXT NOT NULL DEFAULT '',
                        state_code TEXT NOT NULL DEFAULT '',
                        pincode TEXT NOT NULL DEFAULT '',
                        opening_balance_paise INTEGER NOT NULL DEFAULT 0,
                        active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        UNIQUE(name, phone)
                    );
                    CREATE TABLE products (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL COLLATE NOCASE,
                        brand_name TEXT NOT NULL DEFAULT '',
                        generic_name TEXT NOT NULL DEFAULT '',
                        composition TEXT NOT NULL DEFAULT '',
                        manufacturer_id INTEGER REFERENCES manufacturers(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                        unit_id INTEGER REFERENCES units(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                        hsn_code TEXT NOT NULL DEFAULT '',
                        gst_percent REAL NOT NULL DEFAULT 0 CHECK (gst_percent >= 0 AND gst_percent <= 100),
                        schedule TEXT NOT NULL DEFAULT '',
                        barcode TEXT NOT NULL DEFAULT '',
                        reorder_level REAL NOT NULL DEFAULT 0 CHECK (reorder_level >= 0),
                        active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        UNIQUE(name, brand_name, composition)
                    );
                    CREATE INDEX idx_customers_name ON customers(name);
                    CREATE INDEX idx_suppliers_name ON suppliers(name);
                    CREATE INDEX idx_products_name ON products(name);
                    CREATE INDEX idx_products_barcode ON products(barcode) WHERE barcode <> '';
                    CREATE INDEX idx_manufacturers_name ON manufacturers(name);
                    INSERT INTO schema_migrations (version) VALUES (2);
                    PRAGMA user_version = 2;
                    COMMIT;",
                )
                .map_err(|error| format!("Apply master-data schema migration for {start_year}–{}: {error}", start_year + 1))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read financial-year schema version: {error}"))?;
        if version < 3 {
            connection.execute_batch(
                "BEGIN IMMEDIATE;
                ALTER TABLE suppliers ADD COLUMN trade_name TEXT NOT NULL DEFAULT '';
                ALTER TABLE suppliers ADD COLUMN contact_person TEXT NOT NULL DEFAULT '';
                ALTER TABLE suppliers ADD COLUMN payment_terms_days INTEGER NOT NULL DEFAULT 0 CHECK (payment_terms_days >= 0 AND payment_terms_days <= 3650);
                ALTER TABLE suppliers ADD COLUMN credit_limit_paise INTEGER NOT NULL DEFAULT 0 CHECK (credit_limit_paise >= 0);
                CREATE TABLE supplier_manufacturers (
                    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    manufacturer_id INTEGER NOT NULL REFERENCES manufacturers(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY (supplier_id, manufacturer_id)
                );
                CREATE INDEX idx_supplier_manufacturers_manufacturer ON supplier_manufacturers(manufacturer_id, supplier_id);
                CREATE TABLE supplier_products (
                    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    product_id INTEGER NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    supplier_product_code TEXT NOT NULL DEFAULT '',
                    preferred INTEGER NOT NULL DEFAULT 0 CHECK (preferred IN (0, 1)),
                    min_order_qty REAL NOT NULL DEFAULT 0 CHECK (min_order_qty >= 0),
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY (supplier_id, product_id)
                );
                CREATE INDEX idx_supplier_products_product ON supplier_products(product_id, supplier_id);
                INSERT OR IGNORE INTO schema_migrations (version) VALUES (3);
                PRAGMA user_version = 3;
                COMMIT;"
            ).map_err(|error| format!("Apply supplier relationship migration: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read financial-year schema version: {error}"))?;
        if version < 4 {
            connection.execute_batch(
                "BEGIN IMMEDIATE;
                CREATE TABLE purchase_counters (
                    financial_year_start INTEGER PRIMARY KEY,
                    last_number INTEGER NOT NULL DEFAULT 0 CHECK (last_number >= 0)
                );
                CREATE TABLE purchase_documents (
                    id INTEGER PRIMARY KEY,
                    purchase_no TEXT NOT NULL UNIQUE,
                    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    supplier_name_snap TEXT NOT NULL,
                    supplier_gstin_snap TEXT NOT NULL DEFAULT '',
                    supplier_invoice_no TEXT NOT NULL,
                    invoice_date TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'posted' CHECK (status IN ('posted','reversed')),
                    taxable_paise INTEGER NOT NULL CHECK (taxable_paise >= 0),
                    cgst_paise INTEGER NOT NULL CHECK (cgst_paise >= 0),
                    sgst_paise INTEGER NOT NULL CHECK (sgst_paise >= 0),
                    igst_paise INTEGER NOT NULL CHECK (igst_paise >= 0),
                    total_paise INTEGER NOT NULL CHECK (total_paise >= 0),
                    notes TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE UNIQUE INDEX idx_purchase_supplier_invoice
                    ON purchase_documents(supplier_id, supplier_invoice_no COLLATE NOCASE)
                    WHERE supplier_invoice_no <> '';
                CREATE INDEX idx_purchase_date ON purchase_documents(invoice_date, id);
                CREATE TABLE batches (
                    id INTEGER PRIMARY KEY,
                    product_id INTEGER NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    batch_no TEXT NOT NULL COLLATE NOCASE,
                    expiry_date TEXT NOT NULL,
                    mrp_paise INTEGER NOT NULL CHECK (mrp_paise >= 0),
                    purchase_rate_paise INTEGER NOT NULL CHECK (purchase_rate_paise >= 0),
                    sale_rate_paise INTEGER NOT NULL CHECK (sale_rate_paise >= 0),
                    qty_on_hand_milli INTEGER NOT NULL DEFAULT 0 CHECK (qty_on_hand_milli >= 0),
                    free_qty_on_hand_milli INTEGER NOT NULL DEFAULT 0 CHECK (free_qty_on_hand_milli >= 0),
                    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(product_id, batch_no, expiry_date)
                );
                CREATE INDEX idx_batches_product_expiry ON batches(product_id, expiry_date, batch_no);
                CREATE TABLE purchase_items (
                    id INTEGER PRIMARY KEY,
                    purchase_id INTEGER NOT NULL REFERENCES purchase_documents(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    line_no INTEGER NOT NULL,
                    product_id INTEGER NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    batch_id INTEGER NOT NULL REFERENCES batches(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    product_name_snap TEXT NOT NULL,
                    hsn_snap TEXT NOT NULL DEFAULT '',
                    gst_basis_points INTEGER NOT NULL CHECK (gst_basis_points BETWEEN 0 AND 10000),
                    batch_no_snap TEXT NOT NULL,
                    expiry_date_snap TEXT NOT NULL,
                    quantity_milli INTEGER NOT NULL CHECK (quantity_milli > 0),
                    free_quantity_milli INTEGER NOT NULL DEFAULT 0 CHECK (free_quantity_milli >= 0),
                    rate_paise INTEGER NOT NULL CHECK (rate_paise >= 0),
                    discount_basis_points INTEGER NOT NULL DEFAULT 0 CHECK (discount_basis_points BETWEEN 0 AND 10000),
                    taxable_paise INTEGER NOT NULL CHECK (taxable_paise >= 0),
                    cgst_paise INTEGER NOT NULL CHECK (cgst_paise >= 0),
                    sgst_paise INTEGER NOT NULL CHECK (sgst_paise >= 0),
                    igst_paise INTEGER NOT NULL CHECK (igst_paise >= 0),
                    mrp_paise INTEGER NOT NULL CHECK (mrp_paise >= 0),
                    sale_rate_paise INTEGER NOT NULL CHECK (sale_rate_paise >= 0),
                    UNIQUE(purchase_id, line_no)
                );
                CREATE INDEX idx_purchase_items_product ON purchase_items(product_id, purchase_id);
                CREATE TABLE stock_movements (
                    id INTEGER PRIMARY KEY,
                    batch_id INTEGER NOT NULL REFERENCES batches(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    movement_type TEXT NOT NULL CHECK (movement_type IN ('purchase','purchase_reversal','sale','sale_return','adjustment')),
                    reference_type TEXT NOT NULL,
                    reference_id INTEGER NOT NULL,
                    quantity_change_milli INTEGER NOT NULL,
                    free_quantity_change_milli INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE INDEX idx_stock_movements_batch ON stock_movements(batch_id, id);
                INSERT OR IGNORE INTO schema_migrations(version) VALUES (4);
                PRAGMA user_version = 4;
                COMMIT;"
            ).map_err(|error| format!("Apply purchase and inventory migration: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read financial-year schema version: {error}"))?;
        if version < 5 {
            connection.execute_batch(
                "BEGIN IMMEDIATE;
                CREATE TABLE sales_counters (
                    financial_year_start INTEGER PRIMARY KEY,
                    last_number INTEGER NOT NULL DEFAULT 0 CHECK (last_number >= 0)
                );
                CREATE TABLE sales_invoices (
                    id INTEGER PRIMARY KEY,
                    invoice_no TEXT NOT NULL UNIQUE,
                    customer_id INTEGER NOT NULL REFERENCES customers(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    customer_name_snap TEXT NOT NULL,
                    customer_gstin_snap TEXT NOT NULL DEFAULT '',
                    customer_state_code_snap TEXT NOT NULL DEFAULT '',
                    invoice_date TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'posted' CHECK(status IN ('posted','reversed')),
                    taxable_paise INTEGER NOT NULL CHECK(taxable_paise >= 0),
                    cgst_paise INTEGER NOT NULL CHECK(cgst_paise >= 0),
                    sgst_paise INTEGER NOT NULL CHECK(sgst_paise >= 0),
                    igst_paise INTEGER NOT NULL CHECK(igst_paise >= 0),
                    total_paise INTEGER NOT NULL CHECK(total_paise >= 0),
                    received_paise INTEGER NOT NULL CHECK(received_paise >= 0),
                    payment_mode TEXT NOT NULL,
                    notes TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE INDEX idx_sales_invoice_date ON sales_invoices(invoice_date,id);
                CREATE INDEX idx_sales_invoice_customer ON sales_invoices(customer_id,invoice_date);
                CREATE TABLE sales_invoice_items (
                    id INTEGER PRIMARY KEY,
                    invoice_id INTEGER NOT NULL REFERENCES sales_invoices(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    line_no INTEGER NOT NULL,
                    product_id INTEGER NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    product_name_snap TEXT NOT NULL,
                    hsn_snap TEXT NOT NULL DEFAULT '',
                    gst_basis_points INTEGER NOT NULL CHECK(gst_basis_points BETWEEN 0 AND 10000),
                    quantity_milli INTEGER NOT NULL CHECK(quantity_milli > 0),
                    rate_paise INTEGER NOT NULL CHECK(rate_paise >= 0),
                    discount_basis_points INTEGER NOT NULL CHECK(discount_basis_points BETWEEN 0 AND 10000),
                    taxable_paise INTEGER NOT NULL CHECK(taxable_paise >= 0),
                    cgst_paise INTEGER NOT NULL CHECK(cgst_paise >= 0),
                    sgst_paise INTEGER NOT NULL CHECK(sgst_paise >= 0),
                    igst_paise INTEGER NOT NULL CHECK(igst_paise >= 0),
                    UNIQUE(invoice_id,line_no)
                );
                CREATE TABLE sales_invoice_batches (
                    id INTEGER PRIMARY KEY,
                    invoice_item_id INTEGER NOT NULL REFERENCES sales_invoice_items(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    batch_id INTEGER NOT NULL REFERENCES batches(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    quantity_milli INTEGER NOT NULL CHECK(quantity_milli > 0),
                    batch_no_snap TEXT NOT NULL,
                    expiry_date_snap TEXT NOT NULL
                );
                CREATE TABLE sales_payments (
                    id INTEGER PRIMARY KEY,
                    invoice_id INTEGER NOT NULL REFERENCES sales_invoices(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                    amount_paise INTEGER NOT NULL CHECK(amount_paise >= 0),
                    payment_mode TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                INSERT OR IGNORE INTO schema_migrations(version) VALUES (5);
                PRAGMA user_version = 5;
                COMMIT;"
            ).map_err(|error| format!("Apply sales invoice migration: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read financial-year schema version: {error}"))?;

        if version < 6 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                    ALTER TABLE sales_payments
                        ADD COLUMN payment_date TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_payments
                        ADD COLUMN reference_no TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_payments
                        ADD COLUMN notes TEXT NOT NULL DEFAULT '';

                    UPDATE sales_payments
                    SET payment_date = substr(created_at, 1, 10)
                    WHERE payment_date = '';

                    CREATE INDEX idx_sales_payments_invoice_date
                        ON sales_payments(invoice_id, payment_date, id);

                    INSERT OR IGNORE INTO schema_migrations(version)
                        VALUES (6);

                    PRAGMA user_version = 6;
                    COMMIT;",
                )
                .map_err(|error| format!("Apply payment receipt migration: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read financial-year schema version: {error}"))?;

        if version < 7 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;

                    ALTER TABLE sales_invoice_items
                        ADD COLUMN manufacturer_snap TEXT NOT NULL DEFAULT '';

                    ALTER TABLE sales_invoice_items
                        ADD COLUMN unit_snap TEXT NOT NULL DEFAULT '';

                    ALTER TABLE sales_invoice_batches
                        ADD COLUMN mrp_paise_snap INTEGER NOT NULL DEFAULT 0
                        CHECK (mrp_paise_snap >= 0);

                    INSERT OR IGNORE INTO schema_migrations(version)
                        VALUES (7);

                    PRAGMA user_version = 7;
                    COMMIT;",
                )
                .map_err(|error| format!("Apply invoice snapshot migration: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read invoice schema version: {error}"))?;

        if version < 8 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;

                    ALTER TABLE sales_invoice_items
                        ADD COLUMN free_quantity_milli INTEGER NOT NULL DEFAULT 0
                        CHECK (free_quantity_milli >= 0);

                    CREATE TABLE sales_invoice_free_batches (
                        id INTEGER PRIMARY KEY,
                        invoice_item_id INTEGER NOT NULL
                            REFERENCES sales_invoice_items(id)
                            ON UPDATE CASCADE ON DELETE RESTRICT,
                        batch_id INTEGER NOT NULL
                            REFERENCES batches(id)
                            ON UPDATE CASCADE ON DELETE RESTRICT,
                        quantity_milli INTEGER NOT NULL
                            CHECK (quantity_milli > 0),
                        batch_no_snap TEXT NOT NULL,
                        expiry_date_snap TEXT NOT NULL,
                        mrp_paise_snap INTEGER NOT NULL DEFAULT 0
                            CHECK (mrp_paise_snap >= 0)
                    );

                    CREATE INDEX idx_sales_invoice_free_batches_item
                        ON sales_invoice_free_batches(invoice_item_id);

                    INSERT OR IGNORE INTO schema_migrations(version)
                        VALUES (8);

                    PRAGMA user_version = 8;
                    COMMIT;",
                )
                .map_err(|error| format!("Apply free quantity migration: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read invoice schema version: {error}"))?;

        if version < 9 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;

                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_legal_name_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_trade_name_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_gstin_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_drug_license_1_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_drug_license_2_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_address_line1_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_address_line2_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_city_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_state_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_state_code_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_pincode_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_phone_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN seller_email_snap TEXT NOT NULL DEFAULT '';

                    ALTER TABLE sales_invoices
                        ADD COLUMN customer_drug_license_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN customer_address_line1_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN customer_address_line2_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN customer_city_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN customer_state_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN customer_pincode_snap TEXT NOT NULL DEFAULT '';
                    ALTER TABLE sales_invoices
                        ADD COLUMN customer_phone_snap TEXT NOT NULL DEFAULT '';

                    INSERT OR IGNORE INTO schema_migrations(version)
                        VALUES (9);

                    PRAGMA user_version = 9;
                    COMMIT;",
                )
                .map_err(|error| format!("Apply invoice identity snapshot migration: {error}"))?;
        }

        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("Re-read invoice schema version: {error}"))?;

        if version < 10 {
            // Check existing columns so this migration also works on
            // databases where some or all customer snapshots already exist.
            let existing_columns = {
                let mut statement = connection
                    .prepare("PRAGMA table_info(sales_invoices)")
                    .map_err(|error| format!("Inspect sales invoice columns: {error}"))?;

                let rows = statement
                    .query_map([], |row| row.get::<_, String>(1))
                    .map_err(|error| format!("Read sales invoice columns: {error}"))?;

                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("Collect sales invoice columns: {error}"))?
            };

            let customer_columns = [
                ("customer_drug_license_snap", "TEXT NOT NULL DEFAULT ''"),
                ("customer_address_line1_snap", "TEXT NOT NULL DEFAULT ''"),
                ("customer_address_line2_snap", "TEXT NOT NULL DEFAULT ''"),
                ("customer_city_snap", "TEXT NOT NULL DEFAULT ''"),
                ("customer_state_snap", "TEXT NOT NULL DEFAULT ''"),
                ("customer_pincode_snap", "TEXT NOT NULL DEFAULT ''"),
                ("customer_phone_snap", "TEXT NOT NULL DEFAULT ''"),
            ];

            let mut migration_sql = String::from("BEGIN IMMEDIATE;\n");

            for (column, definition) in customer_columns {
                if !existing_columns.iter().any(|existing| existing == column) {
                    migration_sql.push_str(&format!(
                        "ALTER TABLE sales_invoices ADD COLUMN {column} {definition};\n"
                    ));
                }
            }

            migration_sql.push_str(
                "INSERT OR IGNORE INTO schema_migrations(version) VALUES (10);
                 PRAGMA user_version = 10;
                 COMMIT;",
            );

            connection
                .execute_batch(&migration_sql)
                .map_err(|error| format!("Apply customer snapshot migration: {error}"))?;
        }

        let stored_year: Option<i32> = connection
            .query_row(
                "SELECT financial_year_start FROM aster_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("Verify financial-year database {start_year}: {error}"))?;
        if stored_year != Some(start_year) {
            return Err(format!(
                "Financial-year database at {} does not match financial year {start_year}–{}.",
                path.display(),
                start_year + 1
            ));
        }
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|error| {
                format!(
                    "Check {start_year}–{} database integrity: {error}",
                    start_year + 1
                )
            })?;
        if integrity != "ok" {
            return Err(format!(
                "SQLite integrity check failed for {start_year}–{}: {integrity}",
                start_year + 1
            ));
        }
        Ok(path)
    }

    pub(crate) fn financial_years(&self) -> Result<Vec<i32>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT start_year FROM financial_years ORDER BY start_year DESC")
            .map_err(|error| format!("Read financial years: {error}"))?;
        let years = statement
            .query_map([], |row| row.get(0))
            .map_err(|error| format!("Read financial years: {error}"))?
            .collect::<Result<Vec<i32>, _>>()
            .map_err(|error| format!("Read financial years: {error}"))?;
        Ok(years)
    }

    pub(crate) fn create_financial_year(&self, start_year: i32) -> Result<(), String> {
        validate_financial_year(start_year)?;

        let year_directory = self
            .data_dir
            .join(format!("{start_year}-{}", start_year + 1));

        let mut global = self.connection()?;
        let transaction = global
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Start financial-year creation: {e}"))?;

        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM financial_years WHERE start_year = ?1
                )",
                [start_year],
                |row| row.get(0),
            )
            .map_err(|e| format!("Check financial year: {e}"))?;

        if exists {
            return Err(format!(
                "Financial year {start_year}–{} already exists.",
                start_year + 1
            ));
        }

        if year_directory.exists() {
            return Err(format!(
                "The data folder for {start_year}–{} already exists but is not registered. It has been left untouched. Inspect or back it up before proceeding.",
                start_year + 1
            ));
        }

        let previous_year: Option<i32> = transaction
            .query_row(
                "SELECT MAX(start_year)
                FROM financial_years
                WHERE start_year < ?1",
                [start_year],
                |row| row.get(0),
            )
            .map_err(|e| format!("Find previous financial year: {e}"))?;

        let result = (|| {
            self.initialize_year_database(start_year)?;

            if let Some(previous_year) = previous_year {
                self.carry_forward_year(previous_year, start_year)?;
            }

            transaction
                .execute(
                    "INSERT INTO financial_years (start_year) VALUES (?1)",
                    [start_year],
                )
                .map_err(|e| format!("Register financial year: {e}"))?;

            transaction
                .commit()
                .map_err(|e| format!("Finish financial-year creation: {e}"))?;

            Ok(())
        })();

        if result.is_err() {
            // Only remove the new folder. The previous year's data is untouched.
            let _ = fs::remove_dir_all(&year_directory);
        }

        result
    }

    pub(crate) fn delete_financial_year(
        &self,
        start_year: i32,
        current_fy: i32,
    ) -> Result<PathBuf, String> {
        validate_financial_year(start_year)?;
        if start_year == current_fy {
            return Err("The current financial year cannot be deleted.".into());
        }

        let years = self.financial_years()?;

        if !years.contains(&start_year) {
            return Err(format!(
                "Financial year {start_year}–{} is not registered.",
                start_year + 1
            ));
        }

        if years.len() <= 1 {
            return Err("At least one financial year must remain.".into());
        }

        let year_directory = self
            .data_dir
            .join(format!("{start_year}-{}", start_year + 1));

        if !year_directory.is_dir() {
            return Err("The financial-year data folder does not exist.".into());
        }

        let stamp = unique_stamp();
        let backup_path = self.data_dir.join(format!(
            "{start_year}-{}-before-deletion-{stamp}.zip",
            start_year + 1
        ));

        // Back up the global database and every registered financial year.
        self.backup_to_path(&backup_path)?;

        let archive_parent = self.data_dir.join("deleted");
        fs::create_dir_all(&archive_parent)
            .map_err(|e| format!("Create deleted-year archive folder: {e}"))?;

        let archived_directory =
            archive_parent.join(format!("{start_year}-{}-{stamp}", start_year + 1));

        // Move the original folder instead of permanently deleting it.
        fs::rename(&year_directory, &archived_directory)
            .map_err(|e| format!("Archive financial-year data: {e}"))?;

        let result = (|| {
            let mut connection = self.connection()?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| format!("Start financial-year deletion: {e}"))?;

            let deleted = transaction
                .execute(
                    "DELETE FROM financial_years WHERE start_year = ?1",
                    [start_year],
                )
                .map_err(|e| format!("Remove financial-year registration: {e}"))?;

            if deleted != 1 {
                return Err("Financial-year registration was not found.".into());
            }

            transaction
                .commit()
                .map_err(|e| format!("Finish financial-year deletion: {e}"))?;

            Ok(())
        })();

        if let Err(error) = result {
            if let Err(restore_error) = fs::rename(&archived_directory, &year_directory) {
                return Err(format!(
                    "{error}. Also failed to restore the original data folder: {restore_error}. \
                    Your archived data remains at {}",
                    archived_directory.display()
                ));
            }
            return Err(error);
        }

        Ok(backup_path)
    }

    fn carry_forward_year(&self, previous_year: i32, new_year: i32) -> Result<(), String> {
        drop(self.year_connection(previous_year)?);

        let source_path = self.year_database_path(previous_year);
        let target_path = self.year_database_path(new_year);
        let source_text = source_path.to_string_lossy().into_owned();

        let mut target = Connection::open(&target_path)
            .map_err(|e| format!("Open new financial-year database: {e}"))?;

        configure_connection(&target)?;

        target
            .execute(
                "ATTACH DATABASE ?1 AS previous_year",
                [source_text.as_str()],
            )
            .map_err(|e| format!("Attach previous financial year: {e}"))?;

        let transaction = target
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Start carry-forward: {e}"))?;

        let result = transaction.execute_batch(
            "
            DELETE FROM units;

            INSERT INTO company_settings (
                id, legal_name, trade_name, gstin,
                drug_license_1, drug_license_2,
                address_line1, address_line2, city,
                state, state_code, pincode, phone, email,
                invoice_prefix, bank_name, bank_account_name,
                bank_account_number, bank_ifsc, updated_at
            )
            SELECT
                id, legal_name, trade_name, gstin,
                drug_license_1, drug_license_2,
                address_line1, address_line2, city,
                state, state_code, pincode, phone, email,
                invoice_prefix, bank_name, bank_account_name,
                bank_account_number, bank_ifsc, updated_at
            FROM previous_year.company_settings;

            INSERT INTO manufacturers (
                id, name, contact_person, phone,
                email, address, active, created_at
            )
            SELECT
                id, name, contact_person, phone,
                email, address, active, created_at
            FROM previous_year.manufacturers;

            INSERT INTO units (id, name, short_name, active)
            SELECT id, name, short_name, active
            FROM previous_year.units;

            INSERT INTO customers (
                id, name, phone, email, gstin, drug_license,
                address_line1, address_line2, city, state,
                state_code, pincode, credit_limit_paise,
                opening_balance_paise, active, created_at, updated_at
            )
            SELECT
                id, name, phone, email, gstin, drug_license,
                address_line1, address_line2, city, state,
                state_code, pincode, credit_limit_paise,
                opening_balance_paise, active, created_at, updated_at
            FROM previous_year.customers;

            INSERT INTO suppliers (
                id, name, phone, email, gstin, drug_license,
                address_line1, address_line2, city, state,
                state_code, pincode, opening_balance_paise,
                active, created_at, updated_at, trade_name,
                contact_person, payment_terms_days, credit_limit_paise
            )
            SELECT
                id, name, phone, email, gstin, drug_license,
                address_line1, address_line2, city, state,
                state_code, pincode, opening_balance_paise,
                active, created_at, updated_at, trade_name,
                contact_person, payment_terms_days, credit_limit_paise
            FROM previous_year.suppliers;

            INSERT INTO products (
                id, name, brand_name, generic_name, composition,
                manufacturer_id, unit_id, hsn_code, gst_percent,
                schedule, barcode, reorder_level, active,
                created_at, updated_at
            )
            SELECT
                id, name, brand_name, generic_name, composition,
                manufacturer_id, unit_id, hsn_code, gst_percent,
                schedule, barcode, reorder_level, active,
                created_at, updated_at
            FROM previous_year.products;

            INSERT INTO supplier_manufacturers (
                supplier_id, manufacturer_id, created_at
            )
            SELECT supplier_id, manufacturer_id, created_at
            FROM previous_year.supplier_manufacturers;

            INSERT INTO supplier_products (
                supplier_id, product_id, supplier_product_code,
                preferred, min_order_qty, created_at
            )
            SELECT
                supplier_id, product_id, supplier_product_code,
                preferred, min_order_qty, created_at
            FROM previous_year.supplier_products;

            INSERT INTO batches (
                id, product_id, batch_no, expiry_date,
                mrp_paise, purchase_rate_paise, sale_rate_paise,
                qty_on_hand_milli, free_qty_on_hand_milli,
                active, created_at
            )
            SELECT
                id, product_id, batch_no, expiry_date,
                mrp_paise, purchase_rate_paise, sale_rate_paise,
                qty_on_hand_milli, free_qty_on_hand_milli,
                active, created_at
            FROM previous_year.batches;

            INSERT INTO stock_movements (
                batch_id, movement_type, reference_type,
                reference_id, quantity_change_milli,
                free_quantity_change_milli
            )
            SELECT
                id, 'adjustment', 'opening_stock', id,
                qty_on_hand_milli, free_qty_on_hand_milli
            FROM previous_year.batches
            WHERE qty_on_hand_milli <> 0
            OR free_qty_on_hand_milli <> 0;

            UPDATE customers
            SET opening_balance_paise =
                opening_balance_paise + COALESCE((
                    SELECT SUM(total_paise - received_paise)
                    FROM previous_year.sales_invoices
                    WHERE customer_id = customers.id
                    AND status = 'posted'
                ), 0);

            UPDATE suppliers
            SET opening_balance_paise =
                opening_balance_paise + COALESCE((
                    SELECT SUM(total_paise)
                    FROM previous_year.purchase_documents
                    WHERE supplier_id = suppliers.id
                    AND status = 'posted'
                ), 0);
            ",
        );

        if let Err(error) = result {
            drop(transaction);
            let _ = target.execute_batch("DETACH DATABASE previous_year");

            return Err(format!(
                "Carry forward from {previous_year}–{} failed: {error}",
                previous_year + 1
            ));
        }

        transaction
            .commit()
            .map_err(|e| format!("Commit carry-forward: {e}"))?;

        target
            .execute_batch("DETACH DATABASE previous_year")
            .map_err(|e| format!("Detach previous year: {e}"))?;

        let integrity: String = target
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|e| format!("Verify new year database: {e}"))?;

        if integrity != "ok" {
            return Err(format!(
                "New financial-year integrity check failed: {integrity}"
            ));
        }

        Ok(())
    }

    pub(crate) fn year_connection(&self, start_year: i32) -> Result<Connection, String> {
        validate_financial_year(start_year)?;
        let path = self
            .data_dir
            .join(format!("{start_year}-{}", start_year + 1))
            .join("aster.sqlite3");
        if !path.is_file() {
            return Err(format!(
                "Financial-year database {start_year}–{} does not exist.",
                start_year + 1
            ));
        }
        let connection = Connection::open(&path)
            .map_err(|error| format!("Open financial-year database {}: {error}", path.display()))?;
        configure_connection(&connection)?;
        let stored_year: i32 = connection
            .query_row(
                "SELECT financial_year_start FROM aster_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("Verify financial-year database: {error}"))?;
        if stored_year != start_year {
            return Err(format!(
                "Financial-year database mismatch: expected {start_year}–{}.",
                start_year + 1
            ));
        }
        Ok(connection)
    }

    pub(crate) fn company_settings(&self, start_year: i32) -> Result<CompanySettings, String> {
        let connection = self.year_connection(start_year)?;
        connection
            .query_row(
                "SELECT legal_name, trade_name, gstin, drug_license_1, drug_license_2,
                    address_line1, address_line2, city, state, state_code, pincode,
                    phone, email, invoice_prefix, bank_name, bank_account_name,
                    bank_account_number, bank_ifsc
             FROM company_settings WHERE id = 1",
                [],
                |row| {
                    Ok(CompanySettings {
                        legal_name: row.get(0)?,
                        trade_name: row.get(1)?,
                        gstin: row.get(2)?,
                        drug_license_1: row.get(3)?,
                        drug_license_2: row.get(4)?,
                        address_line1: row.get(5)?,
                        address_line2: row.get(6)?,
                        city: row.get(7)?,
                        state: row.get(8)?,
                        state_code: row.get(9)?,
                        pincode: row.get(10)?,
                        phone: row.get(11)?,
                        email: row.get(12)?,
                        invoice_prefix: row.get(13)?,
                        bank_name: row.get(14)?,
                        bank_account_name: row.get(15)?,
                        bank_account_number: row.get(16)?,
                        bank_ifsc: row.get(17)?,
                    })
                },
            )
            .optional()
            .map_err(|error| format!("Read company settings: {error}"))
            .map(|settings| settings.unwrap_or_default())
    }

    pub(crate) fn save_company_settings(
        &self,
        start_year: i32,
        settings: &CompanySettings,
    ) -> Result<(), String> {
        if settings.legal_name.trim().is_empty() {
            return Err("Company legal name is required.".into());
        }
        let company_state_code = settings.state_code.trim();
        if !is_valid_state_code(company_state_code) {
            return Err("Select a valid company GST state.".into());
        }
        if state_name(company_state_code) != Some(settings.state.trim()) {
            return Err(
                "Company state name and state code do not match. Select the state again.".into(),
            );
        }
        validate_gstin_state(&settings.gstin, company_state_code, "Company", true)?;
        let connection = self.year_connection(start_year)?;
        connection.execute(
            "INSERT INTO company_settings (
                id, legal_name, trade_name, gstin, drug_license_1, drug_license_2,
                address_line1, address_line2, city, state, state_code, pincode,
                phone, email, invoice_prefix, bank_name, bank_account_name,
                bank_account_number, bank_ifsc, updated_at
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET
                legal_name=excluded.legal_name, trade_name=excluded.trade_name,
                gstin=excluded.gstin, drug_license_1=excluded.drug_license_1,
                drug_license_2=excluded.drug_license_2, address_line1=excluded.address_line1,
                address_line2=excluded.address_line2, city=excluded.city, state=excluded.state,
                state_code=excluded.state_code, pincode=excluded.pincode, phone=excluded.phone,
                email=excluded.email, invoice_prefix=excluded.invoice_prefix,
                bank_name=excluded.bank_name, bank_account_name=excluded.bank_account_name,
                bank_account_number=excluded.bank_account_number, bank_ifsc=excluded.bank_ifsc,
                updated_at=CURRENT_TIMESTAMP",
            params![settings.legal_name.trim(), settings.trade_name.trim(), settings.gstin.trim(),
                settings.drug_license_1.trim(), settings.drug_license_2.trim(),
                settings.address_line1.trim(), settings.address_line2.trim(), settings.city.trim(),
                settings.state.trim(), settings.state_code.trim(), settings.pincode.trim(),
                settings.phone.trim(), settings.email.trim(),
                if settings.invoice_prefix.trim().is_empty() { "INV" } else { settings.invoice_prefix.trim() },
                settings.bank_name.trim(), settings.bank_account_name.trim(),
                settings.bank_account_number.trim(), settings.bank_ifsc.trim()]
        ).map_err(|error| format!("Save company settings: {error}"))?;
        Ok(())
    }

    pub(crate) fn has_users(&self) -> Result<bool, String> {
        let connection = self.connection()?;
        connection
            .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
            .map_err(|error| format!("Check administrator setup: {error}"))
    }

    pub(crate) fn create_admin(&self, username: &str, password: &str) -> Result<(), String> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|error| format!("Hash password: {error}"))?
            .to_string();
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Start administrator setup: {error}"))?;
        let already_set_up: bool = transaction
            .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
            .map_err(|error| format!("Check administrator setup: {error}"))?;
        if already_set_up {
            return Err("Administrator setup has already been completed.".into());
        }
        transaction
            .execute(
                "INSERT INTO users (username, password_hash, role, active) VALUES (?1, ?2, 'admin', 1)",
                params![username, password_hash],
            )
            .map_err(|error| {
                if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                    "That user name already exists.".to_owned()
                } else {
                    format!("Save administrator: {error}")
                }
            })?;
        transaction
            .commit()
            .map_err(|error| format!("Finish administrator setup: {error}"))?;
        Ok(())
    }

    pub(crate) fn verify_user(&self, username: &str, password: &str) -> Result<bool, String> {
        let connection = self.connection()?;
        let password_hash: Option<String> = connection
            .query_row(
                "SELECT password_hash FROM users WHERE username = ?1 AND active = 1",
                [username],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("Read user credentials: {error}"))?;
        let Some(password_hash) = password_hash else {
            return Ok(false);
        };
        let parsed = PasswordHash::new(&password_hash)
            .map_err(|error| format!("Stored password hash is invalid: {error}"))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    pub(crate) fn change_password(
        &self,
        username: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        if !self.verify_user(username, current_password)? {
            return Err("Your current password is incorrect.".into());
        }
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|error| format!("Hash password: {error}"))?
            .to_string();
        let connection = self.connection()?;
        connection
            .execute(
                "UPDATE users SET password_hash = ?1 WHERE username = ?2 AND active = 1",
                params![password_hash, username],
            )
            .map_err(|error| format!("Update password: {error}"))?;
        Ok(())
    }

    pub(crate) fn backup_to_path(&self, destination: &Path) -> Result<(), String> {
        let years = self.financial_years()?;
        let parent = destination.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("Create backup folder: {e}"))?;

        let temp_dir = backup_temp_dir()?;
        let result = (|| {
            let file =
                fs::File::create(destination).map_err(|e| format!("Create backup archive: {e}"))?;
            let mut archive = ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            archive
                .start_file("manifest.txt", options)
                .map_err(|e| format!("Create backup manifest: {e}"))?;

            let manifest = format!(
                "ASTER_BACKUP_V1\n{}\n",
                years
                    .iter()
                    .map(i32::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            archive
                .write_all(manifest.as_bytes())
                .map_err(|e| format!("Write backup manifest: {e}"))?;

            let global_snapshot = temp_dir.join("global.sqlite3");
            create_sqlite_snapshot(&self.path, &global_snapshot)?;
            archive
                .start_file("global.sqlite3", options)
                .map_err(|e| format!("Add global database: {e}"))?;
            let mut source = fs::File::open(&global_snapshot)
                .map_err(|e| format!("Read global snapshot: {e}"))?;
            std::io::copy(&mut source, &mut archive)
                .map_err(|e| format!("Write global snapshot: {e}"))?;

            for year in &years {
                let snapshot = temp_dir.join(format!("{year}.sqlite3"));
                create_sqlite_snapshot(&self.year_database_path(*year), &snapshot)?;

                archive
                    .start_file(format!("years/{year}.sqlite3"), options)
                    .map_err(|e| format!("Add {year} database: {e}"))?;

                let mut source =
                    fs::File::open(snapshot).map_err(|e| format!("Read {year} snapshot: {e}"))?;
                std::io::copy(&mut source, &mut archive)
                    .map_err(|e| format!("Write {year} snapshot: {e}"))?;
            }

            archive
                .finish()
                .map_err(|e| format!("Finish backup archive: {e}"))?;
            Ok(())
        })();

        let _ = fs::remove_dir_all(&temp_dir);
        result
    }

    pub(crate) fn restore_from_path(&self, source: &Path) -> Result<PathBuf, String> {
        let temp_dir = backup_temp_dir()?;

        let result = (|| {
            let file = fs::File::open(source).map_err(|e| format!("Open backup archive: {e}"))?;
            let mut archive =
                ZipArchive::new(file).map_err(|e| format!("Invalid backup archive: {e}"))?;

            let mut manifest = String::new();
            archive
                .by_name("manifest.txt")
                .map_err(|_| "Backup manifest is missing.".to_owned())?
                .read_to_string(&mut manifest)
                .map_err(|e| format!("Read backup manifest: {e}"))?;

            let mut lines = manifest.lines();
            if lines.next() != Some("ASTER_BACKUP_V1") {
                return Err("Unsupported or invalid ASTER backup version.".into());
            }

            let mut years = Vec::new();
            for line in lines {
                if line.is_empty() {
                    continue;
                }
                let year: i32 = line
                    .parse()
                    .map_err(|_| "Invalid financial year in backup manifest.".to_owned())?;
                validate_financial_year(year)?;
                if years.contains(&year) {
                    return Err("Duplicate financial year in backup manifest.".into());
                }
                years.push(year);
            }

            if years.is_empty() {
                return Err("The backup contains no financial years.".into());
            }

            let global_path = temp_dir.join("global.sqlite3");
            extract_backup_entry(&mut archive, "global.sqlite3", &global_path)?;
            validate_global_backup(&global_path, &years)?;

            let mut staged = vec![(global_path, self.path.clone())];

            for year in &years {
                let entry = format!("years/{year}.sqlite3");
                let staged_path = temp_dir.join(format!("{year}.sqlite3"));
                extract_backup_entry(&mut archive, &entry, &staged_path)?;
                validate_year_backup(&staged_path, *year)?;
                staged.push((staged_path, self.year_database_path(*year)));
            }

            // A separate, complete safety backup is created before any live database
            // is changed. It is placed beside the selected backup archive.
            let safety_path = source.with_file_name(format!(
                "ASTER-before-restore-{}.asterbackup",
                unique_stamp()
            ));
            self.backup_to_path(&safety_path)?;

            replace_databases_safely(&staged)?;

            Ok(safety_path)
        })();

        let _ = fs::remove_dir_all(&temp_dir);
        result
    }
    fn year_database_path(&self, start_year: i32) -> PathBuf {
        self.data_dir
            .join(format!("{start_year}-{}", start_year + 1))
            .join("aster.sqlite3")
    }
    pub(crate) fn products(&self, year: i32, search: &str) -> Result<Vec<ProductRecord>, String> {
        let connection = self.year_connection(year)?;
        let pattern = format!("%{}%", search.trim());

        let mut statement = connection
            .prepare(
                "SELECT p.id, p.name, p.brand_name, p.generic_name,
                        p.composition, p.manufacturer_id,
                        COALESCE(m.name, ''), p.unit_id,
                        COALESCE(u.short_name, ''), p.hsn_code,
                        p.gst_percent, p.schedule, p.barcode,
                        p.reorder_level, p.active
                FROM products p
                LEFT JOIN manufacturers m ON m.id = p.manufacturer_id
                LEFT JOIN units u ON u.id = p.unit_id
                WHERE p.name LIKE ?1 COLLATE NOCASE
                    OR p.brand_name LIKE ?1 COLLATE NOCASE
                    OR p.generic_name LIKE ?1 COLLATE NOCASE
                    OR p.composition LIKE ?1 COLLATE NOCASE
                    OR p.barcode LIKE ?1 COLLATE NOCASE
                ORDER BY p.name COLLATE NOCASE
                LIMIT 500",
            )
            .map_err(|e| format!("Prepare product list: {e}"))?;

        let rows = statement
            .query_map([pattern], |row| {
                Ok(ProductRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    brand_name: row.get(2)?,
                    generic_name: row.get(3)?,
                    composition: row.get(4)?,
                    manufacturer_id: row.get(5)?,
                    manufacturer_name: row.get(6)?,
                    unit_id: row.get(7)?,
                    unit_name: row.get(8)?,
                    hsn_code: row.get(9)?,
                    gst_percent: row.get(10)?,
                    schedule: row.get(11)?,
                    barcode: row.get(12)?,
                    reorder_level: row.get(13)?,
                    active: row.get::<_, i64>(14)? != 0,
                })
            })
            .map_err(|e| format!("Query products: {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read products: {e}"))
    }

    pub(crate) fn stock_batches(
        &self,
        year: i32,
        search: &str,
    ) -> Result<Vec<StockBatchRecord>, String> {
        let connection = self.year_connection(year)?;
        let pattern = format!("%{}%", search.trim());

        let mut statement = connection
            .prepare(
                "SELECT p.id, p.name, p.brand_name,
                        COALESCE(u.short_name, ''),
                        p.reorder_level, b.id,
                        COALESCE(b.batch_no, ''),
                        COALESCE(b.expiry_date, ''),
                        COALESCE(b.qty_on_hand_milli, 0),
                        COALESCE(b.free_qty_on_hand_milli, 0),
                        COALESCE(b.purchase_rate_paise, 0),
                        COALESCE(b.sale_rate_paise, 0),
                        COALESCE(b.mrp_paise, 0)
                FROM products p
                LEFT JOIN units u ON u.id = p.unit_id
                LEFT JOIN batches b
                    ON b.product_id = p.id AND b.active = 1
                WHERE p.active = 1 AND p.id IN (
                    SELECT matched.id
                    FROM products matched
                    LEFT JOIN batches matched_batch
                        ON matched_batch.product_id = matched.id
                        AND matched_batch.active = 1
                    WHERE matched.active = 1 AND (
                        matched.name LIKE ?1 COLLATE NOCASE OR
                        matched.brand_name LIKE ?1 COLLATE NOCASE OR
                        matched.generic_name LIKE ?1 COLLATE NOCASE OR
                        matched.composition LIKE ?1 COLLATE NOCASE OR
                        matched.barcode LIKE ?1 COLLATE NOCASE OR
                        COALESCE(matched_batch.batch_no, '')
                            LIKE ?1 COLLATE NOCASE
                    )
                )
                ORDER BY p.name COLLATE NOCASE,
                        b.expiry_date, b.batch_no",
            )
            .map_err(|e| format!("Prepare stock list: {e}"))?;

        let rows = statement
            .query_map([pattern], |row| {
                Ok(StockBatchRecord {
                    product_id: row.get(0)?,
                    product_name: row.get(1)?,
                    brand_name: row.get(2)?,
                    unit_name: row.get(3)?,
                    reorder_level: row.get(4)?,
                    batch_id: row.get(5)?,
                    batch_no: row.get(6)?,
                    expiry_date: row.get(7)?,
                    quantity_milli: row.get(8)?,
                    free_quantity_milli: row.get(9)?,
                    purchase_rate_paise: row.get(10)?,
                    sale_rate_paise: row.get(11)?,
                    mrp_paise: row.get(12)?,
                })
            })
            .map_err(|e| format!("Query stock list: {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read stock list: {e}"))
    }

    pub(crate) fn manufacturers(&self, year: i32) -> Result<Vec<ManufacturerOption>, String> {
        let connection = self.year_connection(year)?;
        let mut statement = connection
            .prepare(
                "SELECT id, name FROM manufacturers
                WHERE active = 1 ORDER BY name COLLATE NOCASE",
            )
            .map_err(|e| format!("Prepare manufacturers: {e}"))?;

        statement
            .query_map([], |row| {
                Ok(ManufacturerOption {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(|e| format!("Query manufacturers: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read manufacturers: {e}"))
    }

    pub(crate) fn create_manufacturer(&self, year: i32, name: &str) -> Result<i64, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Manufacturer name is required.".into());
        }
        let connection = self.year_connection(year)?;
        connection
            .execute("INSERT INTO manufacturers(name) VALUES(?1)", [name])
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    "A manufacturer with this name already exists.".to_owned()
                } else {
                    format!("Create manufacturer: {e}")
                }
            })?;
        Ok(connection.last_insert_rowid())
    }

    pub(crate) fn units(&self, year: i32) -> Result<Vec<UnitOption>, String> {
        let connection = self.year_connection(year)?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, short_name FROM units
                WHERE active = 1 ORDER BY name COLLATE NOCASE",
            )
            .map_err(|e| format!("Prepare units: {e}"))?;

        statement
            .query_map([], |row| {
                Ok(UnitOption {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    short_name: row.get(2)?,
                })
            })
            .map_err(|e| format!("Query units: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read units: {e}"))
    }

    pub(crate) fn save_product(
        &self,
        year: i32,
        id: Option<i64>,
        product: &ProductRecord,
    ) -> Result<(), String> {
        let name = product.name.trim();

        if name.is_empty() {
            return Err("Product name is required.".to_owned());
        }

        if !(0.0..=100.0).contains(&product.gst_percent) {
            return Err("GST must be between 0 and 100.".to_owned());
        }

        if product.reorder_level < 0.0 {
            return Err("Reorder level cannot be negative.".to_owned());
        }

        let connection = self.year_connection(year)?;

        let result = if let Some(id) = id {
            connection.execute(
                "UPDATE products SET
                    name = ?1, brand_name = ?2, generic_name = ?3,
                    composition = ?4, manufacturer_id = ?5, unit_id = ?6,
                    hsn_code = ?7, gst_percent = ?8, schedule = ?9,
                    barcode = ?10, reorder_level = ?11,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ?12",
                params![
                    name,
                    product.brand_name.trim(),
                    product.generic_name.trim(),
                    product.composition.trim(),
                    product.manufacturer_id,
                    product.unit_id,
                    product.hsn_code.trim(),
                    product.gst_percent,
                    product.schedule.trim(),
                    product.barcode.trim(),
                    product.reorder_level,
                    id,
                ],
            )
        } else {
            connection.execute(
                "INSERT INTO products (
                    name, brand_name, generic_name, composition,
                    manufacturer_id, unit_id, hsn_code, gst_percent,
                    schedule, barcode, reorder_level
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    name,
                    product.brand_name.trim(),
                    product.generic_name.trim(),
                    product.composition.trim(),
                    product.manufacturer_id,
                    product.unit_id,
                    product.hsn_code.trim(),
                    product.gst_percent,
                    product.schedule.trim(),
                    product.barcode.trim(),
                    product.reorder_level,
                ],
            )
        };

        result.map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                "A product with the same name, brand and composition already exists.".to_owned()
            } else {
                format!("Save product: {e}")
            }
        })?;

        Ok(())
    }

    pub(crate) fn set_product_active(
        &self,
        year: i32,
        id: i64,
        active: bool,
    ) -> Result<(), String> {
        let connection = self.year_connection(year)?;

        connection
            .execute(
                "UPDATE products
                SET active = ?1, updated_at = CURRENT_TIMESTAMP
                WHERE id = ?2",
                params![i64::from(active), id],
            )
            .map_err(|e| format!("Update product status: {e}"))?;

        Ok(())
    }
    pub(crate) fn customers(&self, year: i32, search: &str) -> Result<Vec<CustomerRecord>, String> {
        let connection = self.year_connection(year)?;
        let pattern = format!("%{}%", search.trim());
        let mut statement = connection
            .prepare(
                "SELECT id, name, phone, email, gstin, drug_license, address_line1,
                    address_line2, city, state, state_code, pincode,
                    credit_limit_paise, opening_balance_paise, active
            FROM customers
            WHERE name LIKE ?1 COLLATE NOCASE OR phone LIKE ?1 COLLATE NOCASE
                OR email LIKE ?1 COLLATE NOCASE OR gstin LIKE ?1 COLLATE NOCASE
                OR city LIKE ?1 COLLATE NOCASE OR drug_license LIKE ?1 COLLATE NOCASE
            ORDER BY name COLLATE NOCASE LIMIT 1000",
            )
            .map_err(|e| format!("Prepare customer list: {e}"))?;
        statement
            .query_map([pattern], |row| {
                Ok(CustomerRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    phone: row.get(2)?,
                    email: row.get(3)?,
                    gstin: row.get(4)?,
                    drug_license: row.get(5)?,
                    address_line1: row.get(6)?,
                    address_line2: row.get(7)?,
                    city: row.get(8)?,
                    state: row.get(9)?,
                    state_code: row.get(10)?,
                    pincode: row.get(11)?,
                    credit_limit_paise: row.get(12)?,
                    opening_balance_paise: row.get(13)?,
                    active: row.get::<_, i64>(14)? != 0,
                })
            })
            .map_err(|e| format!("Query customers: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read customers: {e}"))
    }

    pub(crate) fn save_customer(
        &self,
        year: i32,
        id: Option<i64>,
        customer: &CustomerRecord,
    ) -> Result<(), String> {
        let name = customer.name.trim();
        if name.is_empty() {
            return Err("Customer name is required.".into());
        }
        if customer.credit_limit_paise < 0 {
            return Err("Credit limit cannot be negative.".into());
        }
        let customer_state_code = customer.state_code.trim();
        if !is_valid_state_code(customer_state_code) {
            return Err("Select a valid customer state.".into());
        }
        if state_name(customer_state_code) != Some(customer.state.trim()) {
            return Err(
                "Customer state name and state code do not match. Select the state again.".into(),
            );
        }
        validate_gstin_state(&customer.gstin, customer_state_code, "Customer", false)?;
        let connection = self.year_connection(year)?;
        let result = if let Some(id) = id {
            connection.execute(
                "UPDATE customers SET name=?1, phone=?2, email=?3, gstin=?4, drug_license=?5,
                address_line1=?6, address_line2=?7, city=?8, state=?9, state_code=?10,
                pincode=?11, credit_limit_paise=?12, opening_balance_paise=?13,
                active=?14, updated_at=CURRENT_TIMESTAMP WHERE id=?15",
                params![
                    name,
                    customer.phone.trim(),
                    customer.email.trim(),
                    customer.gstin.trim().to_uppercase(),
                    customer.drug_license.trim(),
                    customer.address_line1.trim(),
                    customer.address_line2.trim(),
                    customer.city.trim(),
                    customer.state.trim(),
                    customer.state_code.trim(),
                    customer.pincode.trim(),
                    customer.credit_limit_paise,
                    customer.opening_balance_paise,
                    i64::from(customer.active),
                    id
                ],
            )
        } else {
            connection.execute(
                "INSERT INTO customers (name, phone, email, gstin, drug_license, address_line1,
                address_line2, city, state, state_code, pincode, credit_limit_paise,
                opening_balance_paise, active) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![name, customer.phone.trim(), customer.email.trim(), customer.gstin.trim().to_uppercase(),
                    customer.drug_license.trim(), customer.address_line1.trim(), customer.address_line2.trim(),
                    customer.city.trim(), customer.state.trim(), customer.state_code.trim(), customer.pincode.trim(),
                    customer.credit_limit_paise, customer.opening_balance_paise, i64::from(customer.active)],
            )
        };
        result.map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                "A customer with the same name and phone already exists.".to_owned()
            } else {
                format!("Save customer: {e}")
            }
        })?;
        Ok(())
    }

    pub(crate) fn suppliers(&self, year: i32, search: &str) -> Result<Vec<SupplierRecord>, String> {
        let connection = self.year_connection(year)?;
        let pattern = format!("%{}%", search.trim());
        let mut statement = connection
            .prepare(
                "SELECT id,name,trade_name,contact_person,phone,email,gstin,drug_license,
             address_line1,address_line2,city,state,state_code,pincode,payment_terms_days,
             credit_limit_paise,opening_balance_paise,active FROM suppliers
             WHERE name LIKE ?1 COLLATE NOCASE OR trade_name LIKE ?1 COLLATE NOCASE
             OR contact_person LIKE ?1 COLLATE NOCASE OR phone LIKE ?1 COLLATE NOCASE
             OR email LIKE ?1 COLLATE NOCASE OR gstin LIKE ?1 COLLATE NOCASE
             OR city LIKE ?1 COLLATE NOCASE OR drug_license LIKE ?1 COLLATE NOCASE
             ORDER BY name COLLATE NOCASE LIMIT 1000",
            )
            .map_err(|e| format!("Prepare supplier list: {e}"))?;
        statement
            .query_map([pattern], |r| {
                Ok(SupplierRecord {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    trade_name: r.get(2)?,
                    contact_person: r.get(3)?,
                    phone: r.get(4)?,
                    email: r.get(5)?,
                    gstin: r.get(6)?,
                    drug_license: r.get(7)?,
                    address_line1: r.get(8)?,
                    address_line2: r.get(9)?,
                    city: r.get(10)?,
                    state: r.get(11)?,
                    state_code: r.get(12)?,
                    pincode: r.get(13)?,
                    payment_terms_days: r.get(14)?,
                    credit_limit_paise: r.get(15)?,
                    opening_balance_paise: r.get(16)?,
                    active: r.get::<_, i64>(17)? != 0,
                })
            })
            .map_err(|e| format!("Query suppliers: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read suppliers: {e}"))
    }

    pub(crate) fn save_supplier(
        &self,
        year: i32,
        id: Option<i64>,
        supplier: &SupplierRecord,
        manufacturer_ids: &[i64],
        product_links: &[SupplierProductLink],
    ) -> Result<i64, String> {
        let name = supplier.name.trim();
        if name.is_empty() {
            return Err("Supplier legal name is required.".into());
        }
        if !(0..=3650).contains(&supplier.payment_terms_days) {
            return Err("Payment terms must be between 0 and 3650 days.".into());
        }
        if supplier.credit_limit_paise < 0 {
            return Err("Credit limit cannot be negative.".into());
        }
        let supplier_state_code = supplier.state_code.trim();
        if !is_valid_state_code(supplier_state_code) {
            return Err("Select a valid supplier state.".into());
        }
        if state_name(supplier_state_code) != Some(supplier.state.trim()) {
            return Err(
                "Supplier state name and state code do not match. Select the state again.".into(),
            );
        }
        validate_gstin_state(&supplier.gstin, supplier_state_code, "Supplier", false)?;
        let mut connection = self.year_connection(year)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Start supplier save: {e}"))?;
        let result = if let Some(id) = id {
            tx.execute("UPDATE suppliers SET name=?1,trade_name=?2,contact_person=?3,phone=?4,email=?5,gstin=?6,drug_license=?7,address_line1=?8,address_line2=?9,city=?10,state=?11,state_code=?12,pincode=?13,payment_terms_days=?14,credit_limit_paise=?15,opening_balance_paise=?16,active=?17,updated_at=CURRENT_TIMESTAMP WHERE id=?18",
                params![name,supplier.trade_name.trim(),supplier.contact_person.trim(),supplier.phone.trim(),supplier.email.trim(),supplier.gstin.trim().to_uppercase(),supplier.drug_license.trim(),supplier.address_line1.trim(),supplier.address_line2.trim(),supplier.city.trim(),supplier.state.trim(),supplier.state_code.trim(),supplier.pincode.trim(),supplier.payment_terms_days,supplier.credit_limit_paise,supplier.opening_balance_paise,i64::from(supplier.active),id]).map(|_|id)
        } else {
            tx.execute("INSERT INTO suppliers(name,trade_name,contact_person,phone,email,gstin,drug_license,address_line1,address_line2,city,state,state_code,pincode,payment_terms_days,credit_limit_paise,opening_balance_paise,active) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                params![name,supplier.trade_name.trim(),supplier.contact_person.trim(),supplier.phone.trim(),supplier.email.trim(),supplier.gstin.trim().to_uppercase(),supplier.drug_license.trim(),supplier.address_line1.trim(),supplier.address_line2.trim(),supplier.city.trim(),supplier.state.trim(),supplier.state_code.trim(),supplier.pincode.trim(),supplier.payment_terms_days,supplier.credit_limit_paise,supplier.opening_balance_paise,i64::from(supplier.active)]).map(|_|tx.last_insert_rowid())
        };
        let saved_id = result.map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                "A supplier with the same name and phone already exists.".to_owned()
            } else {
                format!("Save supplier: {e}")
            }
        })?;
        tx.execute(
            "DELETE FROM supplier_manufacturers WHERE supplier_id=?1",
            [saved_id],
        )
        .map_err(|e| format!("Replace supplier manufacturer links: {e}"))?;
        for manufacturer_id in manufacturer_ids {
            tx.execute("INSERT OR IGNORE INTO supplier_manufacturers(supplier_id,manufacturer_id) VALUES(?1,?2)",params![saved_id,manufacturer_id]).map_err(|e|format!("Link supplier manufacturer: {e}"))?;
        }
        tx.execute(
            "DELETE FROM supplier_products WHERE supplier_id=?1",
            [saved_id],
        )
        .map_err(|e| format!("Replace supplier product links: {e}"))?;
        for link in product_links {
            if !link.min_order_qty.is_finite() || link.min_order_qty < 0.0 {
                return Err("Minimum order quantity must be a valid non-negative number.".into());
            }
            let manufacturer_id: Option<i64> = tx
                .query_row(
                    "SELECT manufacturer_id FROM products WHERE id=?1",
                    [link.product_id],
                    |r| r.get(0),
                )
                .map_err(|e| format!("Read product manufacturer: {e}"))?;
            if let Some(manufacturer_id) = manufacturer_id {
                tx.execute("INSERT OR IGNORE INTO supplier_manufacturers(supplier_id,manufacturer_id) VALUES(?1,?2)",params![saved_id,manufacturer_id]).map_err(|e|format!("Link product manufacturer to supplier: {e}"))?;
            }
            tx.execute("INSERT INTO supplier_products(supplier_id,product_id,supplier_product_code,preferred,min_order_qty) VALUES(?1,?2,?3,?4,?5)",params![saved_id,link.product_id,link.supplier_product_code.trim(),i64::from(link.preferred),link.min_order_qty]).map_err(|e|format!("Link supplier product: {e}"))?;
        }
        tx.commit()
            .map_err(|e| format!("Finish supplier and association save: {e}"))?;
        Ok(saved_id)
    }

    pub(crate) fn set_supplier_active(
        &self,
        year: i32,
        id: i64,
        active: bool,
    ) -> Result<(), String> {
        let connection = self.year_connection(year)?;
        connection
            .execute(
                "UPDATE suppliers SET active=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                params![i64::from(active), id],
            )
            .map_err(|e| format!("Update supplier status: {e}"))?;
        Ok(())
    }

    pub(crate) fn supplier_manufacturer_ids(
        &self,
        year: i32,
        supplier_id: i64,
    ) -> Result<Vec<i64>, String> {
        let connection = self.year_connection(year)?;
        let mut statement=connection.prepare("SELECT manufacturer_id FROM supplier_manufacturers WHERE supplier_id=?1 ORDER BY manufacturer_id").map_err(|e|format!("Prepare supplier manufacturers: {e}"))?;
        statement
            .query_map([supplier_id], |r| r.get(0))
            .map_err(|e| format!("Query supplier manufacturers: {e}"))?
            .collect::<Result<Vec<i64>, _>>()
            .map_err(|e| format!("Read supplier manufacturers: {e}"))
    }

    pub(crate) fn supplier_product_links(
        &self,
        year: i32,
        supplier_id: i64,
    ) -> Result<Vec<SupplierProductLink>, String> {
        let connection = self.year_connection(year)?;
        let mut statement=connection.prepare("SELECT product_id,supplier_product_code,preferred,min_order_qty FROM supplier_products WHERE supplier_id=?1 ORDER BY product_id").map_err(|e|format!("Prepare supplier products: {e}"))?;
        statement
            .query_map([supplier_id], |r| {
                Ok(SupplierProductLink {
                    product_id: r.get(0)?,
                    supplier_product_code: r.get(1)?,
                    preferred: r.get::<_, i64>(2)? != 0,
                    min_order_qty: r.get(3)?,
                })
            })
            .map_err(|e| format!("Query supplier products: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read supplier products: {e}"))
    }

    pub(crate) fn recent_purchases(
        &self,
        year: i32,
        limit: i64,
    ) -> Result<Vec<PurchaseSummary>, String> {
        let connection = self.year_connection(year)?;
        let mut statement = connection.prepare(
            "SELECT id, purchase_no, supplier_name_snap, supplier_invoice_no, invoice_date, total_paise, status
             FROM purchase_documents ORDER BY id DESC LIMIT ?1"
        ).map_err(|e| format!("Prepare recent purchases: {e}"))?;
        statement
            .query_map([limit.clamp(1, 100)], |r| {
                Ok(PurchaseSummary {
                    id: r.get(0)?,
                    purchase_no: r.get(1)?,
                    supplier_name: r.get(2)?,
                    supplier_invoice_no: r.get(3)?,
                    invoice_date: r.get(4)?,
                    total_paise: r.get(5)?,
                    status: r.get(6)?,
                })
            })
            .map_err(|e| format!("Query recent purchases: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Read recent purchases: {e}"))
    }

    pub(crate) fn post_purchase(&self, year: i32, draft: &PurchaseDraft) -> Result<String, String> {
        if draft.supplier_id <= 0 {
            return Err("Select a supplier.".into());
        }
        if draft.supplier_invoice_no.trim().is_empty() {
            return Err("Supplier invoice number is required.".into());
        }
        if !valid_iso_date(&draft.invoice_date) {
            return Err("Invoice date must be a valid YYYY-MM-DD date.".into());
        }
        if draft.lines.is_empty() {
            return Err("Add at least one product line.".into());
        }
        let mut connection = self.year_connection(year)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Start purchase transaction: {e}"))?;
        let supplier: (String, String, String, i64) = tx
            .query_row(
                "SELECT name, gstin, state_code, active FROM suppliers WHERE id=?1",
                [draft.supplier_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| format!("Find supplier: {e}"))?;
        if supplier.3 == 0 {
            return Err(
                "This supplier is inactive. Reactivate it before posting a purchase.".into(),
            );
        }
        let company_state: String = tx
            .query_row(
                "SELECT state_code FROM company_settings WHERE id=1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("Read company GST state: {e}"))?
            .unwrap_or_default();
        validate_gstin_state(&supplier.1, &supplier.2, "Supplier", false)?;
        let gst_type = gst_type_for_states(&company_state, &supplier.2)?;
        let year_counter: i64 = tx
            .query_row(
                "SELECT last_number FROM purchase_counters WHERE financial_year_start=?1",
                [year],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("Read purchase counter: {e}"))?
            .unwrap_or(0);
        let next = year_counter
            .checked_add(1)
            .ok_or("Purchase number counter overflowed.")?;
        let purchase_no = format!("PUR-{year}-{:06}", next);
        let mut computed = Vec::with_capacity(draft.lines.len());
        let mut total_taxable: i64 = 0;
        let mut total_cgst: i64 = 0;
        let mut total_sgst: i64 = 0;
        let mut total_igst: i64 = 0;
        for (index, line) in draft.lines.iter().enumerate() {
            let product_id = if let Some(new_product) = &line.new_product {
                Self::insert_purchase_product(&tx, new_product, index + 1)?
            } else if line.product_id > 0 {
                line.product_id
            } else {
                return Err(format!(
                    "Line {}: select an existing product or create a new one.",
                    index + 1
                ));
            };
            if line.batch_no.trim().is_empty() {
                return Err(format!("Line {}: batch number is required.", index + 1));
            }
            if !valid_iso_date(&line.expiry_date) {
                return Err(format!(
                    "Line {}: enter a valid expiry date (YYYY-MM-DD).",
                    index + 1
                ));
            }
            if line.expiry_date < draft.invoice_date {
                return Err(format!(
                    "Line {}: batch expiry cannot be before the invoice date.",
                    index + 1
                ));
            }
            if line.quantity_milli <= 0 || line.free_quantity_milli < 0 {
                return Err(format!(
                    "Line {}: quantity must be positive and free quantity cannot be negative.",
                    index + 1
                ));
            }
            if line.rate_paise < 0 || line.mrp_paise < 0 || line.sale_rate_paise < 0 {
                return Err(format!("Line {}: rates cannot be negative.", index + 1));
            }
            if !(0..=10000).contains(&line.discount_basis_points) {
                return Err(format!(
                    "Line {}: discount must be from 0% to 100%.",
                    index + 1
                ));
            }
            let product: (String, String, f64, i64, i64) = tx
                .query_row(
                    "SELECT name, hsn_code, gst_percent, active, id FROM products WHERE id=?1",
                    [product_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .map_err(|_e| format!("Line {}: product no longer exists.", index + 1))?;
            if product.3 == 0 {
                return Err(format!("Line {}: '{}' is inactive.", index + 1, product.0));
            }
            if !product.2.is_finite() || !(0.0..=100.0).contains(&product.2) {
                return Err(format!("Line {}: product GST rate is invalid.", index + 1));
            }
            let gst_bp = (product.2 * 100.0).round() as i64;
            let gross = (line.quantity_milli as i128)
                .checked_mul(line.rate_paise as i128)
                .ok_or("Purchase amount overflow.")?
                / 1000;
            let taxable: i64 = ((gross * (10000 - line.discount_basis_points) as i128 + 5000)
                / 10000)
                .try_into()
                .map_err(|_| "Purchase taxable amount overflow.")?;
            let (cgst, sgst, igst) = calculate_gst_split(taxable, gst_bp, gst_type)?;
            total_taxable = total_taxable
                .checked_add(taxable)
                .ok_or("Purchase total overflow.")?;
            total_cgst = total_cgst
                .checked_add(cgst)
                .ok_or("Purchase tax total overflow.")?;
            total_sgst = total_sgst
                .checked_add(sgst)
                .ok_or("Purchase tax total overflow.")?;
            total_igst = total_igst
                .checked_add(igst)
                .ok_or("Purchase tax total overflow.")?;
            computed.push((
                line, product_id, product.0, product.1, gst_bp, taxable, cgst, sgst, igst,
            ));
        }
        let total = total_taxable
            .checked_add(total_cgst)
            .and_then(|v| v.checked_add(total_sgst))
            .and_then(|v| v.checked_add(total_igst))
            .ok_or("Purchase grand total overflow.")?;
        tx.execute("INSERT INTO purchase_counters(financial_year_start,last_number) VALUES(?1,?2) ON CONFLICT(financial_year_start) DO UPDATE SET last_number=excluded.last_number", params![year,next])
            .map_err(|e| format!("Reserve purchase number: {e}"))?;
        tx.execute(
            "INSERT INTO purchase_documents(purchase_no,supplier_id,supplier_name_snap,supplier_gstin_snap,supplier_invoice_no,invoice_date,taxable_paise,cgst_paise,sgst_paise,igst_paise,total_paise,notes)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![purchase_no,draft.supplier_id,supplier.0,supplier.1,draft.supplier_invoice_no.trim(),draft.invoice_date,total_taxable,total_cgst,total_sgst,total_igst,total,draft.notes.trim()]
        ).map_err(|e| if e.to_string().contains("UNIQUE constraint failed") { "This supplier invoice number has already been recorded.".to_owned() } else { format!("Create purchase: {e}") })?;
        let purchase_id = tx.last_insert_rowid();
        for (index, (line, product_id, product_name, hsn, gst_bp, taxable, cgst, sgst, igst)) in
            computed.iter().enumerate()
        {
            let batch_id: i64 = match tx.query_row(
                "SELECT id FROM batches WHERE product_id=?1 AND batch_no=?2 COLLATE NOCASE AND expiry_date=?3",
                params![
                    product_id,
                    line.batch_no.trim(),
                    line.expiry_date,
                ], |r|r.get(0)
            ).optional().map_err(|e|format!("Find batch: {e}"))? {
                Some(id) => {
                    tx.execute("UPDATE batches SET mrp_paise=?1,purchase_rate_paise=?2,sale_rate_paise=?3 WHERE id=?4",
                        params![line.mrp_paise,line.rate_paise,line.sale_rate_paise,id]).map_err(|e|format!("Update batch prices: {e}"))?; id
                },
                None => {
                    tx.execute("INSERT INTO batches(product_id,batch_no,expiry_date,mrp_paise,purchase_rate_paise,sale_rate_paise) VALUES(?1,?2,?3,?4,?5,?6)",
                        params![product_id,line.batch_no.trim(),line.expiry_date,line.mrp_paise,line.rate_paise,line.sale_rate_paise]).map_err(|e|format!("Create batch: {e}"))?;
                    tx.last_insert_rowid()
                }
            };
            tx.execute("INSERT INTO purchase_items(purchase_id,line_no,product_id,batch_id,product_name_snap,hsn_snap,gst_basis_points,batch_no_snap,expiry_date_snap,quantity_milli,free_quantity_milli,rate_paise,discount_basis_points,taxable_paise,cgst_paise,sgst_paise,igst_paise,mrp_paise,sale_rate_paise)
                VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
                params![
                    purchase_id,
                    index as i64 + 1,
                    product_id,
                    batch_id,
                    product_name,
                    hsn,
                    gst_bp,
                    line.batch_no.trim(),
                    line.expiry_date,
                    line.quantity_milli,
                    line.free_quantity_milli,
                    line.rate_paise,
                    line.discount_basis_points,
                    taxable,
                    cgst,
                    sgst,
                    igst,
                    line.mrp_paise,
                    line.sale_rate_paise
                ])
                .map_err(|e|format!("Save purchase line {}: {e}",index+1))?;
            tx.execute("UPDATE batches SET qty_on_hand_milli=qty_on_hand_milli+?1,free_qty_on_hand_milli=free_qty_on_hand_milli+?2 WHERE id=?3",
                params![line.quantity_milli,line.free_quantity_milli,batch_id]).map_err(|e|format!("Apply stock for line {}: {e}",index+1))?;
            tx.execute("INSERT INTO stock_movements(batch_id,movement_type,reference_type,reference_id,quantity_change_milli,free_quantity_change_milli) VALUES(?1,'purchase','purchase',?2,?3,?4)",
                params![batch_id,purchase_id,line.quantity_milli,line.free_quantity_milli]).map_err(|e|format!("Record stock movement for line {}: {e}",index+1))?;
        }
        tx.commit()
            .map_err(|e| format!("Commit purchase and stock: {e}"))?;
        Ok(purchase_no)
    }

    pub(crate) fn record_sales_payment(
        &self,
        year: i32,
        invoice_id: i64,
        amount_paise: i64,
        payment_mode: &str,
        payment_date: &str,
        reference_no: &str,
        notes: &str,
    ) -> Result<i64, String> {
        if invoice_id <= 0 {
            return Err("Select a valid invoice.".into());
        }

        if amount_paise <= 0 {
            return Err("Payment must be greater than zero.".into());
        }

        let mode = payment_mode.trim();

        if !["Cash", "UPI", "Bank transfer", "Cheque"].contains(&mode) {
            return Err("Select a valid payment mode.".into());
        }

        let date = payment_date.trim();

        if !valid_iso_date(date) {
            return Err("Enter a valid payment date in YYYY-MM-DD format.".into());
        }

        let mut connection = self.year_connection(year)?;

        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Start payment transaction: {e}"))?;

        let invoice: Option<(i64, i64, String)> = tx
            .query_row(
                "SELECT total_paise, received_paise, status
                 FROM sales_invoices
                 WHERE id = ?1",
                [invoice_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| format!("Read invoice balance: {e}"))?;

        let Some((total, received, status)) = invoice else {
            return Err("The selected invoice was not found.".into());
        };

        if status != "posted" {
            return Err("Only posted invoices can receive payments.".into());
        }

        if total < 0 || received < 0 || received > total {
            return Err("The invoice has an invalid payment balance.".into());
        }

        let balance = total - received;

        if balance == 0 {
            return Err("This invoice is already fully paid.".into());
        }

        if amount_paise > balance {
            return Err(format!(
                "Payment exceeds the outstanding balance of ₹{:.2}.",
                balance as f64 / 100.0
            ));
        }

        tx.execute(
            "INSERT INTO sales_payments (
                invoice_id,
                amount_paise,
                payment_mode,
                payment_date,
                reference_no,
                notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                invoice_id,
                amount_paise,
                mode,
                date,
                reference_no.trim(),
                notes.trim()
            ],
        )
        .map_err(|e| format!("Save payment receipt: {e}"))?;

        let payment_id = tx.last_insert_rowid();

        let changed = tx
            .execute(
                "UPDATE sales_invoices
                 SET received_paise = received_paise + ?1
                 WHERE id = ?2
                   AND status = 'posted'
                   AND received_paise >= 0
                   AND received_paise + ?1 <= total_paise",
                params![amount_paise, invoice_id],
            )
            .map_err(|e| format!("Update invoice balance: {e}"))?;

        if changed != 1 {
            return Err("The invoice balance changed. Refresh the invoice and try again.".into());
        }

        tx.commit()
            .map_err(|e| format!("Commit payment receipt: {e}"))?;

        Ok(payment_id)
    }

    pub(crate) fn sales_invoice_detail(
        &self,
        year: i32,
        invoice_id: i64,
    ) -> Result<SalesInvoiceDetail, String> {
        if invoice_id <= 0 {
            return Err("Select a valid invoice.".into());
        }

        let connection = self.year_connection(year)?;
        let mut detail = connection
            .query_row(
                "SELECT
                id, invoice_no,
                customer_name_snap, customer_gstin_snap,
                customer_state_code_snap, invoice_date, status,
                taxable_paise, cgst_paise, sgst_paise, igst_paise,
                total_paise, received_paise, payment_mode, notes,

                seller_legal_name_snap, seller_trade_name_snap,
                seller_gstin_snap, seller_drug_license_1_snap,
                seller_drug_license_2_snap,
                seller_address_line1_snap, seller_address_line2_snap,
                seller_city_snap, seller_state_snap,
                seller_state_code_snap, seller_pincode_snap,
                seller_phone_snap, seller_email_snap,

                customer_drug_license_snap,
                customer_address_line1_snap, customer_address_line2_snap,
                customer_city_snap, customer_state_snap,
                customer_pincode_snap, customer_phone_snap

            FROM sales_invoices
            WHERE id = ?1",
                [invoice_id],
                |row| {
                    Ok(SalesInvoiceDetail {
                        invoice_no: row.get(1)?,

                        seller_legal_name: row.get(15)?,
                        seller_trade_name: row.get(16)?,
                        seller_gstin: row.get(17)?,
                        seller_drug_license_1: row.get(18)?,
                        seller_drug_license_2: row.get(19)?,
                        seller_address_line1: row.get(20)?,
                        seller_address_line2: row.get(21)?,
                        seller_city: row.get(22)?,
                        seller_state: row.get(23)?,
                        seller_state_code: row.get(24)?,
                        seller_pincode: row.get(25)?,
                        seller_phone: row.get(26)?,
                        seller_email: row.get(27)?,

                        customer_name: row.get(2)?,
                        customer_gstin: row.get(3)?,
                        customer_state_code: row.get(4)?,
                        customer_drug_license: row.get(28)?,
                        customer_address_line1: row.get(29)?,
                        customer_address_line2: row.get(30)?,
                        customer_city: row.get(31)?,
                        customer_state: row.get(32)?,
                        customer_pincode: row.get(33)?,
                        customer_phone: row.get(34)?,

                        invoice_date: row.get(5)?,
                        status: row.get(6)?,
                        taxable_paise: row.get(7)?,
                        cgst_paise: row.get(8)?,
                        sgst_paise: row.get(9)?,
                        igst_paise: row.get(10)?,
                        total_paise: row.get(11)?,
                        received_paise: row.get(12)?,
                        payment_mode: row.get(13)?,
                        notes: row.get(14)?,

                        ..Default::default()
                    })
                },
            )
            .optional()
            .map_err(|error| format!("Read invoice header: {error}"))?
            .ok_or_else(|| {
                "The selected invoice was not found in this financial year.".to_owned()
            })?;

        let mut item_statement = connection
            .prepare(
                "SELECT
                    id, line_no, product_name_snap, hsn_snap,
                    manufacturer_snap, unit_snap, gst_basis_points,
                    quantity_milli, free_quantity_milli, rate_paise,
                    discount_basis_points, taxable_paise, cgst_paise,
                    sgst_paise, igst_paise
                 FROM sales_invoice_items
                 WHERE invoice_id = ?1
                 ORDER BY line_no, id",
            )
            .map_err(|error| format!("Prepare invoice items: {error}"))?;

        let item_rows = item_statement
            .query_map([invoice_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    SalesInvoiceItemDetail {
                        line_no: row.get(1)?,
                        product_name: row.get(2)?,
                        hsn: row.get(3)?,
                        manufacturer: row.get(4)?,
                        unit: row.get(5)?,
                        gst_basis_points: row.get(6)?,
                        quantity_milli: row.get(7)?,
                        free_quantity_milli: row.get(8)?,
                        rate_paise: row.get(9)?,
                        discount_basis_points: row.get(10)?,
                        taxable_paise: row.get(11)?,
                        cgst_paise: row.get(12)?,
                        sgst_paise: row.get(13)?,
                        igst_paise: row.get(14)?,
                        ..Default::default()
                    },
                ))
            })
            .map_err(|error| format!("Read invoice items: {error}"))?;

        let item_rows = item_rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Collect invoice items: {error}"))?;
        drop(item_statement);

        for (item_id, mut item) in item_rows {
            let mut paid_statement = connection
                .prepare(
                    "SELECT quantity_milli, batch_no_snap, expiry_date_snap, mrp_paise_snap
                     FROM sales_invoice_batches
                     WHERE invoice_item_id = ?1
                     ORDER BY id",
                )
                .map_err(|error| format!("Prepare paid batch allocations: {error}"))?;
            item.paid_batches = paid_statement
                .query_map([item_id], |row| {
                    Ok(SalesInvoiceBatchDetail {
                        quantity_milli: row.get(0)?,
                        batch_no: row.get(1)?,
                        expiry_date: row.get(2)?,
                        mrp_paise: row.get(3)?,
                    })
                })
                .map_err(|error| format!("Read paid batch allocations: {error}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Collect paid batch allocations: {error}"))?;
            drop(paid_statement);

            let mut free_statement = connection
                .prepare(
                    "SELECT quantity_milli, batch_no_snap, expiry_date_snap, mrp_paise_snap
                     FROM sales_invoice_free_batches
                     WHERE invoice_item_id = ?1
                     ORDER BY id",
                )
                .map_err(|error| format!("Prepare free batch allocations: {error}"))?;
            item.free_batches = free_statement
                .query_map([item_id], |row| {
                    Ok(SalesInvoiceBatchDetail {
                        quantity_milli: row.get(0)?,
                        batch_no: row.get(1)?,
                        expiry_date: row.get(2)?,
                        mrp_paise: row.get(3)?,
                    })
                })
                .map_err(|error| format!("Read free batch allocations: {error}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Collect free batch allocations: {error}"))?;

            detail.items.push(item);
        }

        Ok(detail)
    }

    pub(crate) fn post_sales_invoice(
        &self,
        year: i32,
        draft: &SalesInvoiceDraft,
    ) -> Result<String, String> {
        if draft.customer_id <= 0 {
            return Err("Select a customer.".into());
        }
        if !valid_iso_date(&draft.invoice_date) {
            return Err("Invoice date must be a valid YYYY-MM-DD date.".into());
        }
        if draft.lines.is_empty() {
            return Err("Add at least one product line.".into());
        }
        if !(0..=10000).contains(&draft.overall_discount_basis_points) {
            return Err("Overall discount must be between 0% and 100%.".into());
        }
        if draft.amount_received_paise < 0 {
            return Err("Received amount cannot be negative.".into());
        }
        let mut connection = self.year_connection(year)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Start sales transaction: {e}"))?;
        let customer: (
            String,
            String,
            String,
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        ) = tx
            .query_row(
                "SELECT
                    name, gstin, state_code, active,
                    drug_license, address_line1, address_line2,
                    city, state, pincode, phone, email
                FROM customers WHERE id=?1",
                [draft.customer_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                        r.get(9)?,
                        r.get(10)?,
                        r.get(11)?,
                    ))
                },
            )
            .map_err(|_| "Selected customer no longer exists.".to_owned())?;
        if customer.3 == 0 {
            return Err("This customer is inactive.".into());
        }
        let settings: (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        ) = tx
            .query_row(
                "SELECT
                    invoice_prefix, state_code,
                    legal_name, trade_name, gstin,
                    drug_license_1, drug_license_2,
                    address_line1, address_line2,
                    city, state, pincode, phone, email
                FROM company_settings WHERE id=1",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                        r.get(9)?,
                        r.get(10)?,
                        r.get(11)?,
                        r.get(12)?,
                        r.get(13)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("Read invoice settings: {e}"))?
            .unwrap_or_else(|| {
                (
                    "INV".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            });
        validate_gstin_state(&customer.1, &customer.2, "Customer", false)?;
        let gst_type = gst_type_for_states(&settings.1, &customer.2)?;
        let previous: i64 = tx
            .query_row(
                "SELECT last_number FROM sales_counters WHERE financial_year_start=?1",
                [year],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("Read invoice counter: {e}"))?
            .unwrap_or(0);
        let next = previous
            .checked_add(1)
            .ok_or("Invoice number counter overflowed.")?;
        let prefix = if settings.0.trim().is_empty() {
            "INV"
        } else {
            settings.0.trim()
        };
        let invoice_no = format!("{prefix}-{year}-{:06}", next);
        let mut computed: Vec<(
            SalesLineDraft,
            String,
            String,
            i64,
            i64,
            i64,
            i64,
            i64,
            String,
            String,
        )> = Vec::new();
        let mut taxable_total: i64 = 0;
        let mut cgst_total: i64 = 0;
        let mut sgst_total: i64 = 0;
        let mut igst_total: i64 = 0;
        for (i, line) in draft.lines.iter().enumerate() {
            if line.product_id <= 0
                || line.quantity_milli <= 0
                || line.free_quantity_milli < 0
                || line.rate_paise < 0
                || !(0..=10000).contains(&line.discount_basis_points)
            {
                return Err(format!(
                    "Line {}: product, positive quantity, non-negative rate and valid discount are required.",
                    i + 1
                ));
            }
            let product: (String, String, i64, i64, String, String) = tx
                .query_row(
                    "SELECT
                        p.name,
                        p.hsn_code,
                        CAST(ROUND(p.gst_percent * 100) AS INTEGER),
                        p.active,
                        COALESCE(m.name, ''),
                        COALESCE(u.short_name, '')
                    FROM products p
                    LEFT JOIN manufacturers m ON m.id = p.manufacturer_id
                    LEFT JOIN units u ON u.id = p.unit_id
                    WHERE p.id = ?1",
                    [line.product_id],
                    |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                        ))
                    },
                )
                .map_err(|_| format!("Line {}: product no longer exists.", i + 1))?;
            if product.3 == 0 {
                return Err(format!("Line {}: '{}' is inactive.", i + 1, product.0));
            }
            if !(0..=10000).contains(&product.2) {
                return Err(format!("Line {}: GST rate is invalid.", i + 1));
            }
            let gross = (line.quantity_milli as i128)
                .checked_mul(line.rate_paise as i128)
                .ok_or("Invoice amount overflow.")?
                / 1000;
            let line_taxable: i64 = ((gross * (10000 - line.discount_basis_points) as i128 + 5000)
                / 10000)
                .try_into()
                .map_err(|_| "Invoice taxable amount overflow.")?;
            taxable_total = taxable_total
                .checked_add(line_taxable)
                .ok_or("Invoice taxable total overflow.")?;
            computed.push((
                line.clone(),
                product.0,
                product.1,
                product.2,
                line_taxable,
                0,
                0,
                0,
                product.4,
                product.5,
            ));
        }
        let global_discount =
            ((taxable_total as i128 * draft.overall_discount_basis_points as i128 + 5000) / 10000)
                as i64;
        let mut remaining_discount = global_discount;
        let mut remaining_taxable = taxable_total;
        for item in &mut computed {
            let share: i64 = if remaining_taxable <= 0 {
                0
            } else if item.4 == remaining_taxable {
                remaining_discount
            } else {
                (((item.4 as i128 * global_discount as i128 + taxable_total.max(1) as i128 / 2)
                    / taxable_total.max(1) as i128) as i64)
                    .min(item.4)
            };
            let share = share.min(remaining_discount.max(0));
            let net = item.4 - share;
            remaining_discount -= share;
            remaining_taxable -= item.4;
            let (cgst, sgst, igst) = calculate_gst_split(net, item.3, gst_type)?;
            item.4 = net;
            item.5 = cgst;
            item.6 = sgst;
            item.7 = igst;
            cgst_total = cgst_total
                .checked_add(cgst)
                .ok_or("Invoice tax overflow.")?;
            sgst_total = sgst_total
                .checked_add(sgst)
                .ok_or("Invoice tax overflow.")?;
            igst_total = igst_total
                .checked_add(igst)
                .ok_or("Invoice tax overflow.")?;
        }
        let taxable_after_discount = computed.iter().try_fold(0i64, |a, x| {
            a.checked_add(x.4).ok_or("Invoice taxable total overflow.")
        })?;
        let total = taxable_after_discount
            .checked_add(cgst_total)
            .and_then(|v| v.checked_add(sgst_total))
            .and_then(|v| v.checked_add(igst_total))
            .ok_or("Invoice total overflow.")?;
        if draft.amount_received_paise > total {
            return Err(
                "Received amount cannot exceed invoice total; record excess as a separate receipt."
                    .into(),
            );
        }
        tx.execute("INSERT INTO sales_counters(financial_year_start,last_number) VALUES(?1,?2) ON CONFLICT(financial_year_start) DO UPDATE SET last_number=excluded.last_number",params![year,next]).map_err(|e|format!("Reserve invoice number: {e}"))?;
        tx.execute(
            "INSERT INTO sales_invoices (
            invoice_no, customer_id,
            customer_name_snap, customer_gstin_snap,
            customer_state_code_snap,

            seller_legal_name_snap, seller_trade_name_snap,
            seller_gstin_snap, seller_drug_license_1_snap,
            seller_drug_license_2_snap,
            seller_address_line1_snap, seller_address_line2_snap,
            seller_city_snap, seller_state_snap,
            seller_state_code_snap, seller_pincode_snap,
            seller_phone_snap, seller_email_snap,

            customer_drug_license_snap,
            customer_address_line1_snap, customer_address_line2_snap,
            customer_city_snap, customer_state_snap,
            customer_pincode_snap, customer_phone_snap,

            invoice_date, taxable_paise, cgst_paise,
            sgst_paise, igst_paise, total_paise,
            received_paise, payment_mode, notes
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18,
            ?19, ?20, ?21, ?22, ?23, ?24, ?25,
            ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34
        )",
            params![
                invoice_no,
                draft.customer_id,
                customer.0,
                customer.1,
                customer.2,
                settings.2,
                settings.3,
                settings.4,
                settings.5,
                settings.6,
                settings.7,
                settings.8,
                settings.9,
                settings.10,
                settings.1,
                settings.11,
                settings.12,
                settings.13,
                customer.4,
                customer.5,
                customer.6,
                customer.7,
                customer.8,
                customer.9,
                customer.10,
                draft.invoice_date,
                taxable_after_discount,
                cgst_total,
                sgst_total,
                igst_total,
                total,
                draft.amount_received_paise,
                draft.payment_mode.trim(),
                draft.notes.trim()
            ],
        )
        .map_err(|e| format!("Save sales invoice: {e}"))?;
        let invoice_id = tx.last_insert_rowid();
        for (index, item) in computed.iter().enumerate() {
            let line = &item.0;
            tx.execute(
                "INSERT INTO sales_invoice_items (
                        invoice_id, line_no, product_id,
                        product_name_snap, hsn_snap, gst_basis_points,
                        quantity_milli, free_quantity_milli,
                        rate_paise, discount_basis_points,
                        taxable_paise, cgst_paise, sgst_paise, igst_paise,
                        manufacturer_snap, unit_snap
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                        ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
                    )",
                params![
                    invoice_id,
                    index as i64 + 1,
                    line.product_id,
                    item.1,
                    item.2,
                    item.3,
                    line.quantity_milli,
                    line.free_quantity_milli,
                    line.rate_paise,
                    line.discount_basis_points,
                    item.4,
                    item.5,
                    item.6,
                    item.7,
                    item.8,
                    item.9,
                ],
            )
            .map_err(|e| format!("Save invoice line {}: {e}", index + 1))?;
            let item_id = tx.last_insert_rowid();
            let mut needed = line.quantity_milli;
            let mut stmt = tx
                .prepare(
                    "SELECT
                        id,
                        batch_no,
                        expiry_date,
                        qty_on_hand_milli,
                        mrp_paise
                    FROM batches
                    WHERE product_id = ?1
                    AND active = 1
                    AND expiry_date >= ?2
                    AND qty_on_hand_milli > 0
                    ORDER BY expiry_date, id",
                )
                .map_err(|e| format!("Prepare stock allocation: {e}"))?;
            let batches = stmt
                .query_map(params![line.product_id, draft.invoice_date], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                })
                .map_err(|e| format!("Read available stock: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Read stock batches: {e}"))?;
            drop(stmt);
            let available: i64 = batches.iter().try_fold(0i64, |a, b| {
                a.checked_add(b.3).ok_or("Stock quantity overflow.")
            })?;
            if available < needed {
                return Err(format!(
                    "Insufficient unexpired stock for '{}': requested {:.3}, available {:.3}.",
                    item.1,
                    line.quantity_milli as f64 / 1000.0,
                    available as f64 / 1000.0
                ));
            }
            for (batch_id, batch_no, expiry, qty, mrp_paise) in batches {
                if needed <= 0 {
                    break;
                }
                let take = needed.min(qty);
                tx.execute("UPDATE batches SET qty_on_hand_milli=qty_on_hand_milli-?1 WHERE id=?2 AND qty_on_hand_milli>=?1",params![take,batch_id]).map_err(|e|format!("Deduct batch stock: {e}"))?;
                tx.execute(
                    "INSERT INTO sales_invoice_batches (
                        invoice_item_id,
                        batch_id,
                        quantity_milli,
                        batch_no_snap,
                        expiry_date_snap,
                        mrp_paise_snap
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![item_id, batch_id, take, batch_no, expiry, mrp_paise],
                )
                .map_err(|e| format!("Save invoice batch allocation: {e}"))?;
                tx.execute("INSERT INTO stock_movements(batch_id,movement_type,reference_type,reference_id,quantity_change_milli,free_quantity_change_milli) VALUES(?1,'sale','sales_invoice',?2,?3,0)",params![batch_id,invoice_id,-take]).map_err(|e|format!("Record stock movement: {e}"))?;
                needed -= take;
            }
            // Allocate supplier-provided free stock separately.
            let mut free_needed = line.free_quantity_milli;

            if free_needed > 0 {
                let mut stmt = tx
                    .prepare(
                        "SELECT
                            id,
                            batch_no,
                            expiry_date,
                            free_qty_on_hand_milli,
                            mrp_paise
                        FROM batches
                        WHERE product_id = ?1
                        AND active = 1
                        AND expiry_date >= ?2
                        AND free_qty_on_hand_milli > 0
                        ORDER BY expiry_date, id",
                    )
                    .map_err(|e| format!("Prepare free-stock allocation: {e}"))?;

                let free_batches = stmt
                    .query_map(params![line.product_id, draft.invoice_date], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, i64>(3)?,
                            r.get::<_, i64>(4)?,
                        ))
                    })
                    .map_err(|e| format!("Read available free stock: {e}"))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("Read free-stock batches: {e}"))?;

                drop(stmt);

                let free_available: i64 = free_batches.iter().try_fold(0i64, |a, b| {
                    a.checked_add(b.3).ok_or("Free-stock quantity overflow.")
                })?;

                if free_available < free_needed {
                    return Err(format!(
                        "Insufficient supplier-provided free stock for '{}': requested {:.3}, available {:.3}.",
                        item.1,
                        free_needed as f64 / 1000.0,
                        free_available as f64 / 1000.0
                    ));
                }

                for (batch_id, batch_no, expiry, qty, mrp_paise) in free_batches {
                    if free_needed <= 0 {
                        break;
                    }

                    let take = free_needed.min(qty);

                    tx.execute(
                        "UPDATE batches
                        SET free_qty_on_hand_milli = free_qty_on_hand_milli - ?1
                        WHERE id = ?2
                        AND free_qty_on_hand_milli >= ?1",
                        params![take, batch_id],
                    )
                    .map_err(|e| format!("Deduct free batch stock: {e}"))?;

                    tx.execute(
                        "INSERT INTO sales_invoice_free_batches (
                            invoice_item_id,
                            batch_id,
                            quantity_milli,
                            batch_no_snap,
                            expiry_date_snap,
                            mrp_paise_snap
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![item_id, batch_id, take, batch_no, expiry, mrp_paise],
                    )
                    .map_err(|e| format!("Save free batch allocation: {e}"))?;

                    tx.execute(
                        "INSERT INTO stock_movements (
                            batch_id,
                            movement_type,
                            reference_type,
                            reference_id,
                            quantity_change_milli,
                            free_quantity_change_milli
                        ) VALUES (?1, 'sale', 'sales_invoice', ?2, 0, ?3)",
                        params![batch_id, invoice_id, -take],
                    )
                    .map_err(|e| format!("Record free-stock movement: {e}"))?;

                    free_needed -= take;
                }
            }
        }
        if draft.amount_received_paise > 0 {
            tx.execute(
                "INSERT INTO sales_payments (
                    invoice_id,
                    amount_paise,
                    payment_mode,
                    payment_date,
                    reference_no,
                    notes
                ) VALUES (?1, ?2, ?3, ?4, '', ?5)",
                params![
                    invoice_id,
                    draft.amount_received_paise,
                    draft.payment_mode.trim(),
                    draft.invoice_date,
                    draft.notes.trim()
                ],
            )
            .map_err(|e| format!("Record invoice payment: {e}"))?;
        }
        tx.commit()
            .map_err(|e| format!("Commit invoice and stock: {e}"))?;
        Ok(invoice_no)
    }

    pub(crate) fn set_customer_active(
        &self,
        year: i32,
        id: i64,
        active: bool,
    ) -> Result<(), String> {
        let connection = self.year_connection(year)?;
        connection
            .execute(
                "UPDATE customers SET active=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                params![i64::from(active), id],
            )
            .map_err(|e| format!("Update customer status: {e}"))?;
        Ok(())
    }
}

fn valid_iso_date(value: &str) -> bool {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    let (Ok(y), Ok(m), Ok(d)) = (
        parts[0].parse::<i32>(),
        parts[1].parse::<u32>(),
        parts[2].parse::<u32>(),
    ) else {
        return false;
    };
    if !(1..=12).contains(&m) || d == 0 {
        return false;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    d <= max
}

fn configure_connection(connection: &Connection) -> Result<(), String> {
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| format!("Configure SQLite busy timeout: {error}"))?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| format!("Enable database foreign keys: {error}"))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| format!("Configure SQLite journaling: {error}"))?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| format!("Configure SQLite synchronization: {error}"))?;
    Ok(())
}

fn validate_financial_year(start_year: i32) -> Result<(), String> {
    if !(MIN_FINANCIAL_YEAR..=MAX_FINANCIAL_YEAR).contains(&start_year) {
        return Err(format!(
            "Financial year must start between {MIN_FINANCIAL_YEAR} and {MAX_FINANCIAL_YEAR}."
        ));
    }
    Ok(())
}

fn unique_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn backup_temp_dir() -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!(
        "aster-backup-{}-{}",
        std::process::id(),
        unique_stamp()
    ));
    fs::create_dir_all(&path).map_err(|e| format!("Create temporary backup folder: {e}"))?;
    Ok(path)
}

fn create_sqlite_snapshot(source: &Path, destination: &Path) -> Result<(), String> {
    let source_connection = Connection::open(source)
        .map_err(|e| format!("Open source database {}: {e}", source.display()))?;
    let mut destination_connection =
        Connection::open(destination).map_err(|e| format!("Create SQLite snapshot: {e}"))?;

    let backup = Backup::new(&source_connection, &mut destination_connection)
        .map_err(|e| format!("Initialize SQLite backup: {e}"))?;

    backup
        .run_to_completion(100, Duration::from_millis(10), None)
        .map_err(|e| format!("Create SQLite snapshot: {e}"))?;

    drop(backup);

    let integrity: String = destination_connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|e| format!("Verify SQLite snapshot: {e}"))?;

    if integrity != "ok" {
        return Err(format!("Snapshot integrity check failed: {integrity}"));
    }
    Ok(())
}

fn extract_backup_entry(
    archive: &mut ZipArchive<fs::File>,
    entry_name: &str,
    destination: &Path,
) -> Result<(), String> {
    let mut entry = archive
        .by_name(entry_name)
        .map_err(|_| format!("Backup is missing {entry_name}."))?;

    if entry.size() > 2 * 1024 * 1024 * 1024 {
        return Err(format!("{entry_name} exceeds the 2 GB safety limit."));
    }

    let mut output =
        fs::File::create(destination).map_err(|e| format!("Prepare restored database: {e}"))?;

    std::io::copy(&mut entry, &mut output).map_err(|e| format!("Extract {entry_name}: {e}"))?;
    output
        .sync_all()
        .map_err(|e| format!("Flush restored database: {e}"))?;
    Ok(())
}

fn validate_sqlite_file(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|e| format!("Open backup database: {e}"))?;
    let integrity: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|e| format!("Check backup database: {e}"))?;

    if integrity != "ok" {
        return Err(format!(
            "Backup database integrity check failed: {integrity}"
        ));
    }
    Ok(connection)
}

fn validate_global_backup(path: &Path, expected_years: &[i32]) -> Result<(), String> {
    let connection = validate_sqlite_file(path)?;
    let mut statement = connection
        .prepare("SELECT start_year FROM financial_years ORDER BY start_year DESC")
        .map_err(|e| format!("Read backup financial years: {e}"))?;

    let actual = statement
        .query_map([], |row| row.get::<_, i32>(0))
        .map_err(|e| format!("Read backup financial years: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Read backup financial years: {e}"))?;

    let mut expected = expected_years.to_vec();
    expected.sort_unstable_by(|a, b| b.cmp(a));

    if actual != expected {
        return Err("Backup manifest does not match its global database.".into());
    }
    Ok(())
}

fn validate_year_backup(path: &Path, expected_year: i32) -> Result<(), String> {
    let connection = validate_sqlite_file(path)?;
    let stored_year: i32 = connection
        .query_row(
            "SELECT financial_year_start FROM aster_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Verify {expected_year} financial-year database: {e}"))?;

    if stored_year != expected_year {
        return Err(format!(
            "Financial-year database mismatch: expected {expected_year}–{}.",
            expected_year + 1
        ));
    }
    Ok(())
}

fn replace_databases_safely(staged: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    let stamp = unique_stamp();
    let mut replaced: Vec<(PathBuf, PathBuf)> = Vec::new();

    for (index, (staged_path, target_path)) in staged.iter().enumerate() {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Prepare database folder: {e}"))?;
        }

        let old_path = target_path.with_extension(format!("pre-restore-{stamp}-{index}"));
        if target_path.exists() {
            fs::rename(target_path, &old_path)
                .map_err(|e| format!("Protect existing database: {e}"))?;
        }

        if let Err(error) = fs::copy(staged_path, target_path) {
            if old_path.exists() {
                let _ = fs::rename(&old_path, target_path);
            }
            for (old, target) in replaced.iter().rev() {
                let _ = fs::remove_file(target);
                let _ = fs::rename(old, target);
            }
            return Err(format!("Replace database: {error}"));
        }

        if old_path.exists() {
            replaced.push((old_path, target_path.clone()));
        }
    }

    // Old database copies are retained as an additional recovery measure.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_directory(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("aster-{label}-{}-{stamp}", std::process::id()))
    }

    fn remove_directory(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn gst_state_classification_and_split_are_consistent() {
        assert_eq!(
            gst_type_for_states("34", "34").unwrap(),
            GstType::IntraState
        );
        assert_eq!(
            gst_type_for_states("34", "33").unwrap(),
            GstType::InterState
        );
        assert_eq!(
            calculate_gst_split(100_000, 1_800, GstType::IntraState),
            Ok((9_000, 9_000, 0))
        );
        assert_eq!(
            calculate_gst_split(100_000, 1_800, GstType::InterState),
            Ok((0, 0, 18_000))
        );
        assert!(calculate_gst_split(-1, 1_800, GstType::IntraState).is_err());
        assert!(calculate_gst_split(100_000, 10_001, GstType::InterState).is_err());
        assert!(gst_type_for_states("", "34").is_err());
        assert!(gst_type_for_states("34", "XX").is_err());
    }

    #[test]
    fn gstin_state_must_match_selected_state() {
        assert!(validate_gstin_state("34ABCDE1234F1Z5", "34", "Customer", false).is_ok());
        assert!(validate_gstin_state("33ABCDE1234F1Z5", "34", "Customer", false).is_err());
        assert!(validate_gstin_state("", "34", "Customer", false).is_ok());
        assert!(validate_gstin_state("", "34", "Company", true).is_err());
    }

    #[test]
    fn global_database_and_current_year_database_initialize() {
        let root = temporary_directory("initialize");
        let db = Database::open_at(root.clone()).expect("database should initialize");
        assert!(db.path.is_file());
        let years = db.financial_years().expect("financial years should load");
        assert!(!years.is_empty());
        assert!(db.year_database_path(years[0]).is_file());
        assert!(
            db.connection()
                .expect("global database should reopen")
                .is_autocommit()
        );
        drop(db);
        remove_directory(&root);
    }

    #[test]
    fn creating_year_creates_separate_database_and_duplicate_is_rejected() {
        let root = temporary_directory("create-year");
        let db = Database::open_at(root.clone()).expect("database should initialize");
        let year = 2040;
        db.create_financial_year(year)
            .expect("new year should be created");
        assert!(db.year_database_path(year).is_file());
        assert!(
            db.financial_years()
                .expect("years should load")
                .contains(&year)
        );
        assert!(db.create_financial_year(year).is_err());
        drop(db);
        remove_directory(&root);
    }

    #[test]
    fn master_schema_and_company_settings_are_scoped_to_each_year() {
        let root = temporary_directory("master-schema");
        let db = Database::open_at(root.clone()).expect("database should initialize");
        db.create_financial_year(2040)
            .expect("first year should be created");
        db.create_financial_year(2041)
            .expect("second year should be created");

        let mut settings = CompanySettings::default();
        settings.legal_name = "ASTER Medical Wholesale".into();
        settings.gstin = "33ABCDE1234F1Z5".into();
        settings.state = "Tamil Nadu".into();
        settings.state_code = "33".into();
        db.save_company_settings(2040, &settings)
            .expect("settings should save");
        assert_eq!(
            db.company_settings(2040)
                .expect("settings should load")
                .legal_name,
            "ASTER Medical Wholesale"
        );
        assert_eq!(
            db.company_settings(2041)
                .expect("other year should be empty")
                .legal_name,
            ""
        );

        let connection = db.year_connection(2040).expect("year db should open");
        let customers: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='customers'",
                [],
                |r| r.get(0),
            )
            .expect("customers table should exist");
        let products: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='products'",
                [],
                |r| r.get(0),
            )
            .expect("products table should exist");
        assert_eq!(customers, 1);
        assert_eq!(products, 1);
        drop(connection);
        drop(db);
        remove_directory(&root);
    }

    #[test]
    fn invalid_financial_year_is_rejected() {
        assert!(validate_financial_year(1899).is_err());
        assert!(validate_financial_year(9999).is_err());
        assert!(validate_financial_year(2026).is_ok());
    }
}
