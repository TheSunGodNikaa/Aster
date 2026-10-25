use std::{fs, path::PathBuf};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::date::local_date;

const CURRENT_GLOBAL_SCHEMA: i64 = 2;
const CURRENT_YEAR_SCHEMA: i64 = 3;
const MIN_FINANCIAL_YEAR: i32 = 1900;
const MAX_FINANCIAL_YEAR: i32 = 9998;

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
        // Create and validate the year database before registering it globally. If
        // initialization fails, the year selector will not show a broken year.
        self.initialize_year_database(start_year)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Start financial-year creation: {error}"))?;
        transaction
            .execute(
                "INSERT INTO financial_years (start_year) VALUES (?1)",
                [start_year],
            )
            .map_err(|error| {
                if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                    format!(
                        "Financial year {start_year}–{} already exists.",
                        start_year + 1
                    )
                } else {
                    format!(
                        "Register financial year {start_year}–{}: {error}",
                        start_year + 1
                    )
                }
            })?;
        transaction
            .commit()
            .map_err(|error| format!("Finish financial-year creation: {error}"))?;
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

    #[cfg(test)]
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
