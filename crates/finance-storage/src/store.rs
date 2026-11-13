//! The SQLite-backed store.

use std::collections::BTreeMap;
use std::path::Path;

use finance_core::{
    Account, AccountId, Budget, Category, CategoryId, Currency, Ledger, Money, Posting,
    Transaction, TransactionId,
};
use rusqlite::{Connection, OptionalExtension, Row, params};
use time::OffsetDateTime;

use crate::StoreError;
use crate::backup::{self, Backup};
use crate::codec;
use crate::integrity::{self, IntegrityReport};
use crate::schema;

/// One dataset, in one SQLite file.
///
/// The currency is fixed when the store is opened and checked against the file,
/// because a ledger and a dataset in different currencies would silently mix
/// amounts that only look comparable (ADR-0003).
#[derive(Debug)]
pub struct Store {
    connection: Connection,
    currency: Currency,
    dirty: bool,
}

impl Store {
    /// Opens or creates the dataset at `path`.
    ///
    /// Creates the schema if the file is new, and refuses a file written by a
    /// newer build.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] if the file cannot be opened,
    /// [`StoreError::UnsupportedSchema`] if it was written by a newer build, and
    /// [`StoreError::CurrencyMismatch`] if it holds another currency.
    pub fn open(path: impl AsRef<Path>, currency: Currency) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path)?, currency)
    }

    /// Opens an empty dataset held only in memory, for tests and previews.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] if SQLite cannot start, and
    /// [`StoreError::UnsupportedSchema`] or [`StoreError::CurrencyMismatch`] as
    /// [`Self::open`] does.
    pub fn open_in_memory(currency: Currency) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?, currency)
    }

    fn from_connection(mut connection: Connection, currency: Currency) -> Result<Self, StoreError> {
        schema::prepare(&mut connection)?;

        match read_currency(&connection)? {
            Some(stored) if stored != currency => Err(StoreError::CurrencyMismatch {
                stored,
                expected: currency,
            }),
            Some(_) => Ok(Self {
                connection,
                currency,
                dirty: false,
            }),
            None => {
                write_currency(&connection, currency)?;
                Ok(Self {
                    connection,
                    currency,
                    dirty: false,
                })
            }
        }
    }

    /// The currency this dataset is held in.
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// The schema version on disk.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] if the version cannot be read and
    /// [`StoreError::Corrupt`] if it is not a number.
    pub fn schema_version(&self) -> Result<u32, StoreError> {
        schema::current_version(&self.connection)
    }

    /// True when the dataset has changed since it was last backed up (FR-080).
    ///
    /// The app uses this to decide whether the automatic backup on exit has
    /// anything to do.
    pub const fn needs_backup(&self) -> bool {
        self.dirty
    }

    /// Runs `PRAGMA integrity_check` and reports what it finds, changing nothing
    /// (FR-085).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] if the check cannot run.
    pub fn integrity_check(&self) -> Result<IntegrityReport, StoreError> {
        integrity::check(&self.connection)
    }

    /// Writes a consistent copy of the dataset to `path`, which must not exist.
    ///
    /// The copy is switched out of WAL mode, so it is one file with no sidecar
    /// files: a backup that needs a `-wal` beside it is not a backup (FR-081).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::BackupExists`] when `path` is already taken,
    /// [`StoreError::InvalidPath`] for a path that cannot be used as text, and
    /// [`StoreError::Database`] if the copy fails.
    pub fn backup_to(
        &mut self,
        path: impl AsRef<Path>,
        at: OffsetDateTime,
    ) -> Result<Backup, StoreError> {
        let path = path.as_ref();
        if path.exists() {
            return Err(StoreError::BackupExists {
                path: path.to_path_buf(),
            });
        }
        let schema_version = self.schema_version()?;
        let text = path
            .to_str()
            .ok_or_else(|| StoreError::InvalidPath(path.to_path_buf()))?;

        self.connection.execute("VACUUM INTO ?1", params![text])?;

        {
            let copy = Connection::open(path)?;
            copy.query_row("PRAGMA journal_mode = DELETE", [], |_| Ok(()))?;
        }

        self.dirty = false;
        Ok(Backup::new(path.to_path_buf(), at, schema_version))
    }

    /// Writes a backup into `directory`, named from `at`, then prunes the oldest
    /// so at most [`backup::KEEP_BACKUPS`] remain (FR-080, FR-086).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Io`] if the directory cannot be created, and
    /// propagates [`Self::backup_to`] and [`backup::prune`].
    pub fn backup_into(
        &mut self,
        directory: impl AsRef<Path>,
        at: OffsetDateTime,
    ) -> Result<Backup, StoreError> {
        let directory = directory.as_ref();
        std::fs::create_dir_all(directory)?;

        let path = directory.join(backup::file_name(at, self.schema_version()?));
        let written = self.backup_to(&path, at)?;
        backup::prune(directory, backup::KEEP_BACKUPS)?;

        Ok(written)
    }

    /// Reads the whole dataset.
    ///
    /// Soft-deleted transactions are not part of the result: as far as the
    /// domain is concerned they do not exist. They stay on disk for undo and for
    /// the audit trail (FR-014).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Corrupt`] for a row this build cannot interpret,
    /// and propagates [`StoreError::Ledger`] when the rows are individually
    /// readable but do not form a valid ledger — a duplicate identifier, a
    /// missing parent, an unbalanced transaction.
    pub fn load(&self) -> Result<Ledger, StoreError> {
        let mut ledger = Ledger::new(self.currency);

        for account in self.accounts()? {
            ledger.add_account(account)?;
        }
        for category in self.categories()? {
            ledger.add_category(category)?;
        }
        for transaction in self.transactions()? {
            ledger.record(transaction)?;
        }
        for budget in self.budgets()? {
            ledger.set_budget(budget.category(), budget.amount(), budget.effective_from())?;
        }

        Ok(ledger)
    }

    /// Replaces the live dataset with `ledger`, in one transaction.
    ///
    /// Soft-deleted transactions and their postings are left in place, so a save
    /// never resurrects or destroys them. Everything else is rewritten: the
    /// store is a projection of the ledger, and the ledger is the truth.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::LedgerCurrency`] when the ledger is in another
    /// currency, and [`StoreError::Database`] if the write fails. A failed write
    /// leaves the previous dataset untouched.
    pub fn save(&mut self, ledger: &Ledger) -> Result<(), StoreError> {
        if ledger.currency() != self.currency {
            return Err(StoreError::LedgerCurrency {
                ledger: ledger.currency(),
                store: self.currency,
            });
        }

        let transaction = self.connection.transaction()?;
        // Every live row is replaced, so foreign keys are checked once at
        // commit instead of during the deletes and inserts.
        transaction.execute_batch("PRAGMA defer_foreign_keys = ON")?;

        transaction.execute("DELETE FROM budgets", [])?;
        transaction.execute("DELETE FROM transactions WHERE deleted_at IS NULL", [])?;
        transaction.execute("DELETE FROM categories", [])?;
        transaction.execute("DELETE FROM accounts", [])?;

        insert_accounts(&transaction, ledger)?;
        insert_categories(&transaction, ledger)?;
        insert_transactions(&transaction, ledger)?;
        insert_budgets(&transaction, ledger)?;
        write_currency(&transaction, self.currency)?;

        transaction.commit()?;
        self.dirty = true;
        Ok(())
    }

    /// Marks a transaction as deleted without removing it.
    ///
    /// Idempotent: deleting an already deleted transaction succeeds. The row
    /// survives for undo and is excluded from [`load`](Self::load).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::UnknownTransaction`] when the identifier was never
    /// stored, and [`StoreError::Database`] if the write fails.
    pub fn soft_delete_transaction(
        &mut self,
        id: TransactionId,
        at: OffsetDateTime,
    ) -> Result<(), StoreError> {
        let key = codec::uuid(id.as_uuid());
        let updated = self.connection.execute(
            "UPDATE transactions SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![key.as_str(), codec::timestamp(at)?],
        )?;
        if updated > 0 {
            self.dirty = true;
            return Ok(());
        }

        let existing: Option<Option<String>> = self
            .connection
            .query_row(
                "SELECT deleted_at FROM transactions WHERE id = ?1",
                params![key.as_str()],
                |row| row.get(0),
            )
            .optional()?;

        match existing {
            Some(_) => Ok(()),
            None => Err(StoreError::UnknownTransaction(id)),
        }
    }

    /// Brings a soft-deleted transaction back.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::UnknownTransaction`] when the identifier was never
    /// stored, and [`StoreError::Database`] if the write fails.
    pub fn restore_transaction(&mut self, id: TransactionId) -> Result<(), StoreError> {
        let key = codec::uuid(id.as_uuid());
        let updated = self.connection.execute(
            "UPDATE transactions SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
            params![key.as_str()],
        )?;
        if updated > 0 {
            self.dirty = true;
            return Ok(());
        }

        let exists: Option<i64> = self
            .connection
            .query_row(
                "SELECT 1 FROM transactions WHERE id = ?1",
                params![key.as_str()],
                |row| row.get(0),
            )
            .optional()?;

        match exists {
            Some(_) => Ok(()),
            None => Err(StoreError::UnknownTransaction(id)),
        }
    }

    /// Identifiers of the soft-deleted transactions, most recently deleted last.
    ///
    /// Exposed so a later export can purge them deliberately (Q-34) and so tests
    /// can prove a save does not quietly discard them.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] if the rows cannot be read and
    /// [`StoreError::Corrupt`] if an identifier is not a UUID.
    pub fn deleted_transactions(&self) -> Result<Vec<TransactionId>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id FROM transactions WHERE deleted_at IS NOT NULL ORDER BY deleted_at, id",
        )?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        ids.iter()
            .map(|text| {
                Ok(TransactionId::from_uuid(codec::parse_uuid(
                    text,
                    "transactions",
                )?))
            })
            .collect()
    }

    fn accounts(&self) -> Result<Vec<Account>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, kind, currency_code, currency_exponent, archived
             FROM accounts ORDER BY id",
        )?;
        let rows: Vec<RawAccount> = statement
            .query_map([], RawAccount::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut accounts = Vec::with_capacity(rows.len());
        for row in rows {
            let id = AccountId::from_uuid(codec::parse_uuid(&row.id, "accounts")?);
            let kind = codec::parse_account_kind(&row.kind, "accounts")?;
            let currency = codec::parse_currency(&row.code, row.exponent, "accounts")?;
            let mut account = Account::new(id, row.name, kind, currency);
            if row.archived != 0 {
                account.archive();
            }
            accounts.push(account);
        }

        Ok(accounts)
    }

    fn categories(&self) -> Result<Vec<Category>, StoreError> {
        // Parents before children, because the domain refuses a child whose
        // parent it has not seen yet.
        let mut statement = self.connection.prepare(
            "SELECT id, name, kind, parent FROM categories
             ORDER BY parent IS NOT NULL, id",
        )?;
        let rows: Vec<RawCategory> = statement
            .query_map([], RawCategory::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut categories = Vec::with_capacity(rows.len());
        for row in rows {
            let id = CategoryId::from_uuid(codec::parse_uuid(&row.id, "categories")?);
            let kind = codec::parse_category_kind(&row.kind, "categories")?;
            let mut category = Category::new(id, row.name, kind);
            if let Some(parent) = row.parent {
                let parent = CategoryId::from_uuid(codec::parse_uuid(&parent, "categories")?);
                category = category.with_parent(parent);
            }
            categories.push(category);
        }

        Ok(categories)
    }

    fn transactions(&self) -> Result<Vec<Transaction>, StoreError> {
        let mut postings = self.postings()?;

        let mut statement = self.connection.prepare(
            "SELECT id, date, kind, payee, note, source, source_profile FROM transactions
             WHERE deleted_at IS NULL ORDER BY rowid",
        )?;
        let rows: Vec<RawTransaction> = statement
            .query_map([], RawTransaction::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut transactions = Vec::with_capacity(rows.len());
        for row in rows {
            let key = codec::parse_uuid(&row.id, "transactions")?;
            let date = codec::parse_date(&row.date, "transactions")?;
            let kind = codec::parse_transaction_kind(&row.kind, "transactions")?;
            let source = codec::parse_transaction_source(
                &row.source,
                row.source_profile.as_deref(),
                "transactions",
            )?;
            let items = postings.remove(&row.id).unwrap_or_default();

            let mut transaction =
                Transaction::new(TransactionId::from_uuid(key), date, kind, items)?
                    .with_source(source);
            if let Some(payee) = row.payee {
                transaction = transaction.with_payee(payee);
            }
            if let Some(note) = row.note {
                transaction = transaction.with_note(note);
            }
            transactions.push(transaction);
        }

        Ok(transactions)
    }

    fn postings(&self) -> Result<BTreeMap<String, Vec<Posting>>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT transaction_id, target_kind, target_id, amount_minor FROM postings
             WHERE transaction_id IN (SELECT id FROM transactions WHERE deleted_at IS NULL)
             ORDER BY transaction_id, position",
        )?;
        let rows: Vec<RawPosting> = statement
            .query_map([], RawPosting::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut grouped: BTreeMap<String, Vec<Posting>> = BTreeMap::new();
        for row in rows {
            let target = codec::decode_target(&row.kind, row.id.as_deref(), "postings")?;
            grouped
                .entry(row.transaction)
                .or_default()
                .push(Posting::new(
                    target,
                    Money::from_minor(row.minor, self.currency),
                ));
        }

        Ok(grouped)
    }

    fn budgets(&self) -> Result<Vec<Budget>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT category, effective_from, amount_minor FROM budgets
             ORDER BY category, effective_from",
        )?;
        let rows: Vec<RawBudget> = statement
            .query_map([], RawBudget::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut budgets = Vec::with_capacity(rows.len());
        for row in rows {
            let category = CategoryId::from_uuid(codec::parse_uuid(&row.category, "budgets")?);
            let period = codec::parse_period(&row.period, "budgets")?;
            budgets.push(Budget::new(
                category,
                Money::from_minor(row.minor, self.currency),
                period,
            ));
        }

        Ok(budgets)
    }
}

/// Reads the currency recorded in the dataset, if it has one yet.
fn read_currency(connection: &Connection) -> Result<Option<Currency>, StoreError> {
    let code: Option<String> = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'currency.code'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let exponent: Option<String> = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'currency.exponent'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    match (code, exponent) {
        (None, None) => Ok(None),
        (Some(code), Some(exponent)) => {
            let exponent: i64 = exponent.parse().map_err(|_| StoreError::Corrupt {
                table: "metadata",
                detail: format!("currency exponent {exponent:?} is not a number"),
            })?;
            Ok(Some(codec::parse_currency(&code, exponent, "metadata")?))
        }
        _ => Err(StoreError::Corrupt {
            table: "metadata",
            detail: "the currency is only half recorded".to_owned(),
        }),
    }
}

/// Records the dataset's currency. Cheap, and keeps the metadata true even if
/// the file was edited by hand.
fn write_currency(connection: &Connection, currency: Currency) -> Result<(), StoreError> {
    for (key, value) in [
        ("currency.code", currency.code().to_owned()),
        ("currency.exponent", currency.exponent().to_string()),
    ] {
        connection.execute(
            "INSERT INTO metadata (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
    }
    Ok(())
}

fn insert_accounts(connection: &Connection, ledger: &Ledger) -> Result<(), StoreError> {
    let mut insert = connection.prepare(
        "INSERT INTO accounts (id, name, kind, currency_code, currency_exponent, archived)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;

    for account in ledger.accounts() {
        insert.execute(params![
            codec::uuid(account.id().as_uuid()),
            account.name(),
            codec::account_kind(account.kind()),
            account.currency().code(),
            i64::from(account.currency().exponent()),
            i64::from(u8::from(account.is_archived())),
        ])?;
    }

    Ok(())
}

fn insert_categories(connection: &Connection, ledger: &Ledger) -> Result<(), StoreError> {
    let mut insert = connection
        .prepare("INSERT INTO categories (id, name, kind, parent) VALUES (?1, ?2, ?3, ?4)")?;

    let mut categories: Vec<&Category> = ledger.categories().collect();
    categories.sort_by_key(|category| (category.parent().is_some(), category.id()));

    for category in categories {
        insert.execute(params![
            codec::uuid(category.id().as_uuid()),
            category.name(),
            codec::category_kind(category.kind()),
            category.parent().map(|id| codec::uuid(id.as_uuid())),
        ])?;
    }

    Ok(())
}

fn insert_transactions(connection: &Connection, ledger: &Ledger) -> Result<(), StoreError> {
    let mut insert = connection.prepare(
        "INSERT INTO transactions (id, date, kind, payee, note, source, source_profile, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
    )?;
    let mut insert_posting = connection.prepare(
        "INSERT INTO postings (transaction_id, position, target_kind, target_id, amount_minor)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for transaction in ledger.transactions() {
        let id = codec::uuid(transaction.id().as_uuid());
        let (source, profile) = codec::transaction_source(transaction.source());
        insert.execute(params![
            id.as_str(),
            codec::date(transaction.date()),
            codec::transaction_kind(transaction.kind()),
            transaction.payee(),
            transaction.note(),
            source,
            profile,
        ])?;

        for (position, posting) in transaction.postings().iter().enumerate() {
            let (kind, target) = codec::encode_target(posting.target());
            insert_posting.execute(params![
                id.as_str(),
                position as i64,
                kind,
                target,
                posting.amount().minor(),
            ])?;
        }
    }

    Ok(())
}

fn insert_budgets(connection: &Connection, ledger: &Ledger) -> Result<(), StoreError> {
    let mut insert = connection.prepare(
        "INSERT INTO budgets (category, effective_from, amount_minor) VALUES (?1, ?2, ?3)",
    )?;

    for budget in ledger.budgets() {
        insert.execute(params![
            codec::uuid(budget.category().as_uuid()),
            codec::period(budget.effective_from()),
            budget.amount().minor(),
        ])?;
    }

    Ok(())
}

/// A row exactly as SQLite returns it, before any interpretation.
///
/// Naming the shape keeps column order in one place and stops the conversion
/// from becoming an unreadable tuple.
struct RawAccount {
    id: String,
    name: String,
    kind: String,
    code: String,
    exponent: i64,
    archived: i64,
}

impl RawAccount {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            code: row.get(3)?,
            exponent: row.get(4)?,
            archived: row.get(5)?,
        })
    }
}

struct RawCategory {
    id: String,
    name: String,
    kind: String,
    parent: Option<String>,
}

impl RawCategory {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            parent: row.get(3)?,
        })
    }
}

struct RawTransaction {
    id: String,
    date: String,
    kind: String,
    payee: Option<String>,
    note: Option<String>,
    source: String,
    source_profile: Option<String>,
}

impl RawTransaction {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            date: row.get(1)?,
            kind: row.get(2)?,
            payee: row.get(3)?,
            note: row.get(4)?,
            source: row.get(5)?,
            source_profile: row.get(6)?,
        })
    }
}

struct RawPosting {
    transaction: String,
    kind: String,
    id: Option<String>,
    minor: i64,
}

impl RawPosting {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            transaction: row.get(0)?,
            kind: row.get(1)?,
            id: row.get(2)?,
            minor: row.get(3)?,
        })
    }
}

struct RawBudget {
    category: String,
    period: String,
    minor: i64,
}

impl RawBudget {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            category: row.get(0)?,
            period: row.get(1)?,
            minor: row.get(2)?,
        })
    }
}
