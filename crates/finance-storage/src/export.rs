//! Exporting the dataset to plain CSV files.
//!
//! FR-083 asks for an export that can be read without this app, so the format is
//! deliberately boring: one file per table, a header row of column names, and a
//! manifest that names the version and the row counts. No encoding tricks, no
//! binary blobs, no grouping separators — money is written as plain decimal text
//! next to its currency code, so a spreadsheet reads it without knowing anything
//! about minor units.
//!
//! The export is a snapshot, not a live view: it is written into a staging
//! directory and renamed into place, so a crash leaves either the previous state
//! or nothing rather than a half-written set of files that looks complete.
//!
//! Soft-deleted transactions are exported with their deletion time (Q-34). The
//! export is the only place they are ever purged from, and the person reading the
//! files is the one who decides that; hiding them would make the export a lossy
//! copy of the dataset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use finance_core::{Currency, Money};
use rusqlite::Connection;
use time::{OffsetDateTime, UtcOffset};

use crate::StoreError;
use crate::codec;
use crate::schema;

/// The version of the CSV layout this build writes.
///
/// A reader can compare it with the manifest instead of guessing from the column
/// names. It changes only when a column changes meaning or disappears.
pub const FORMAT_VERSION: u32 = 1;

/// The manifest's file name.
pub const MANIFEST: &str = "manifest.csv";

/// One file an export contains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExportFile {
    /// Accounts, including archived ones.
    Accounts,
    /// Categories, with their parent when they have one.
    Categories,
    /// Transactions, including soft-deleted ones.
    Transactions,
    /// The postings that make each transaction balance.
    Postings,
    /// Budget revisions.
    Budgets,
    /// Remembered classification rules.
    Rules,
    /// The manifest: format version, schema version, time and row counts.
    Manifest,
}

impl ExportFile {
    /// Every file an export contains, in the order they are written.
    pub const ALL: [Self; 7] = [
        Self::Accounts,
        Self::Categories,
        Self::Transactions,
        Self::Postings,
        Self::Budgets,
        Self::Rules,
        Self::Manifest,
    ];

    /// The file's name inside the export directory.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Accounts => "accounts.csv",
            Self::Categories => "categories.csv",
            Self::Transactions => "transactions.csv",
            Self::Postings => "postings.csv",
            Self::Budgets => "budgets.csv",
            Self::Rules => "rules.csv",
            Self::Manifest => MANIFEST,
        }
    }
}

/// An export that was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Export {
    path: PathBuf,
    created_at: OffsetDateTime,
    rows: BTreeMap<ExportFile, usize>,
}

impl Export {
    /// The directory holding the files.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The moment the export was taken, as recorded in the directory name and the
    /// manifest.
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// How many data rows a file holds, not counting its header.
    ///
    /// The UI uses this to say "112 transactions" without reopening the files.
    pub fn rows(&self, file: ExportFile) -> usize {
        self.rows.get(&file).copied().unwrap_or_default()
    }

    /// Every file and its row count, in a stable order.
    pub fn files(&self) -> impl Iterator<Item = (ExportFile, usize)> + '_ {
        self.rows.iter().map(|(file, rows)| (*file, *rows))
    }
}

/// The directory name for an export taken at `at`.
///
/// Fixed-width UTC, like a backup's name, so a directory of exports sorts by age.
/// The `finance-export-` prefix and the `Z` suffix make it obvious that the
/// directory is this app's output and not something the user put there.
pub fn directory_name(at: OffsetDateTime) -> String {
    let at = at.to_offset(UtcOffset::UTC);
    format!(
        "finance-export-{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

/// Writes every table into a new directory under `parent`.
///
/// The files are written into `<name>.partial` first and the directory is renamed
/// when every file is complete, so an interrupted export is never mistaken for a
/// finished one.
pub(crate) fn write(
    connection: &Connection,
    currency: Currency,
    parent: &Path,
    created_at: OffsetDateTime,
) -> Result<Export, StoreError> {
    let name = directory_name(created_at);
    let target = parent.join(&name);
    if target.exists() {
        return Err(StoreError::ExportExists { path: target });
    }

    let staging = parent.join(format!("{name}.partial"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;

    match write_files(&staging, connection, currency, created_at) {
        Ok(rows) => {
            std::fs::rename(&staging, &target)?;
            Ok(Export {
                path: target,
                created_at,
                rows,
            })
        }
        Err(error) => {
            // Leaving a partial export behind would make the next run's failure
            // look like a success to anyone listing the directory.
            let _ = std::fs::remove_dir_all(&staging);
            Err(error)
        }
    }
}

fn write_files(
    directory: &Path,
    connection: &Connection,
    currency: Currency,
    created_at: OffsetDateTime,
) -> Result<BTreeMap<ExportFile, usize>, StoreError> {
    let mut rows = BTreeMap::new();
    rows.insert(ExportFile::Accounts, write_accounts(directory, connection)?);
    rows.insert(
        ExportFile::Categories,
        write_categories(directory, connection)?,
    );
    rows.insert(
        ExportFile::Transactions,
        write_transactions(directory, connection)?,
    );
    rows.insert(
        ExportFile::Postings,
        write_postings(directory, connection, currency)?,
    );
    rows.insert(
        ExportFile::Budgets,
        write_budgets(directory, connection, currency)?,
    );
    rows.insert(ExportFile::Rules, write_rules(directory, connection)?);
    rows.insert(
        ExportFile::Manifest,
        write_manifest(directory, connection, currency, created_at, &rows)?,
    );

    Ok(rows)
}

/// Opens a writer for one file of the export.
fn writer(directory: &Path, file: ExportFile) -> Result<csv::Writer<std::fs::File>, StoreError> {
    Ok(csv::WriterBuilder::new().from_path(directory.join(file.name()))?)
}

fn write_accounts(directory: &Path, connection: &Connection) -> Result<usize, StoreError> {
    let mut writer = writer(directory, ExportFile::Accounts)?;
    writer.write_record([
        "id",
        "name",
        "kind",
        "currency_code",
        "currency_exponent",
        "archived",
    ])?;

    let mut statement = connection.prepare(
        "SELECT id, name, kind, currency_code, currency_exponent, archived
         FROM accounts ORDER BY id",
    )?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let kind: String = row.get(2)?;
        let code: String = row.get(3)?;
        let exponent: i64 = row.get(4)?;
        let archived: i64 = row.get(5)?;
        let exponent = exponent.to_string();

        writer.write_record([
            id.as_str(),
            name.as_str(),
            kind.as_str(),
            code.as_str(),
            exponent.as_str(),
            if archived == 0 { "false" } else { "true" },
        ])?;
        count += 1;
    }

    writer.flush()?;
    Ok(count)
}

fn write_categories(directory: &Path, connection: &Connection) -> Result<usize, StoreError> {
    let mut writer = writer(directory, ExportFile::Categories)?;
    writer.write_record(["id", "name", "kind", "parent_id"])?;

    let mut statement = connection.prepare(
        "SELECT id, name, kind, parent FROM categories
         ORDER BY parent IS NOT NULL, id",
    )?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let kind: String = row.get(2)?;
        let parent: Option<String> = row.get(3)?;

        writer.write_record([
            id.as_str(),
            name.as_str(),
            kind.as_str(),
            parent.as_deref().unwrap_or(""),
        ])?;
        count += 1;
    }

    writer.flush()?;
    Ok(count)
}

fn write_transactions(directory: &Path, connection: &Connection) -> Result<usize, StoreError> {
    let mut writer = writer(directory, ExportFile::Transactions)?;
    writer.write_record([
        "id",
        "date",
        "kind",
        "payee",
        "note",
        "source",
        "source_profile",
        "deleted_at",
    ])?;

    // Soft-deleted rows are included: the export is the copy the user keeps, and
    // a deletion marker is history, not noise (Q-34).
    let mut statement = connection.prepare(
        "SELECT id, date, kind, payee, note, source, source_profile, deleted_at
         FROM transactions ORDER BY date, id",
    )?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let date: String = row.get(1)?;
        let kind: String = row.get(2)?;
        let payee: Option<String> = row.get(3)?;
        let note: Option<String> = row.get(4)?;
        let source: String = row.get(5)?;
        let profile: Option<String> = row.get(6)?;
        let deleted_at: Option<String> = row.get(7)?;

        writer.write_record([
            id.as_str(),
            date.as_str(),
            kind.as_str(),
            payee.as_deref().unwrap_or(""),
            note.as_deref().unwrap_or(""),
            source.as_str(),
            profile.as_deref().unwrap_or(""),
            deleted_at.as_deref().unwrap_or(""),
        ])?;
        count += 1;
    }

    writer.flush()?;
    Ok(count)
}

fn write_postings(
    directory: &Path,
    connection: &Connection,
    currency: Currency,
) -> Result<usize, StoreError> {
    let mut writer = writer(directory, ExportFile::Postings)?;
    writer.write_record([
        "transaction_id",
        "position",
        "target_kind",
        "target_id",
        "amount",
        "currency_code",
    ])?;

    let mut statement = connection.prepare(
        "SELECT transaction_id, position, target_kind, target_id, amount_minor
         FROM postings ORDER BY transaction_id, position",
    )?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let transaction: String = row.get(0)?;
        let position: i64 = row.get(1)?;
        let kind: String = row.get(2)?;
        let id: Option<String> = row.get(3)?;
        let minor: i64 = row.get(4)?;
        let position = position.to_string();
        let amount = Money::from_minor(minor, currency).to_plain_string();

        writer.write_record([
            transaction.as_str(),
            position.as_str(),
            kind.as_str(),
            id.as_deref().unwrap_or(""),
            amount.as_str(),
            currency.code(),
        ])?;
        count += 1;
    }

    writer.flush()?;
    Ok(count)
}

fn write_budgets(
    directory: &Path,
    connection: &Connection,
    currency: Currency,
) -> Result<usize, StoreError> {
    let mut writer = writer(directory, ExportFile::Budgets)?;
    writer.write_record(["category_id", "effective_from", "amount", "currency_code"])?;

    let mut statement = connection.prepare(
        "SELECT category, effective_from, amount_minor FROM budgets
         ORDER BY category, effective_from",
    )?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let category: String = row.get(0)?;
        let period: String = row.get(1)?;
        let minor: i64 = row.get(2)?;
        let amount = Money::from_minor(minor, currency).to_plain_string();

        writer.write_record([
            category.as_str(),
            period.as_str(),
            amount.as_str(),
            currency.code(),
        ])?;
        count += 1;
    }

    writer.flush()?;
    Ok(count)
}

fn write_rules(directory: &Path, connection: &Connection) -> Result<usize, StoreError> {
    let mut writer = writer(directory, ExportFile::Rules)?;
    writer.write_record([
        "key_kind",
        "merchant",
        "pattern",
        "counterparty",
        "target_kind",
        "target_id",
    ])?;

    let mut statement = connection.prepare(
        "SELECT key_kind, merchant, pattern, counterparty, target_kind, target_id
         FROM rules ORDER BY key_kind, merchant, pattern, counterparty",
    )?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let key_kind: String = row.get(0)?;
        let merchant: String = row.get(1)?;
        let pattern: String = row.get(2)?;
        let counterparty: String = row.get(3)?;
        let target_kind: String = row.get(4)?;
        let target_id: String = row.get(5)?;

        writer.write_record([
            key_kind.as_str(),
            merchant.as_str(),
            pattern.as_str(),
            counterparty.as_str(),
            target_kind.as_str(),
            target_id.as_str(),
        ])?;
        count += 1;
    }

    writer.flush()?;
    Ok(count)
}

fn write_manifest(
    directory: &Path,
    connection: &Connection,
    currency: Currency,
    created_at: OffsetDateTime,
    rows: &BTreeMap<ExportFile, usize>,
) -> Result<usize, StoreError> {
    let mut entries = vec![
        ("format_version".to_owned(), FORMAT_VERSION.to_string()),
        (
            "schema_version".to_owned(),
            schema::current_version(connection)?.to_string(),
        ),
        ("created_at".to_owned(), codec::timestamp(created_at)?),
        ("currency_code".to_owned(), currency.code().to_owned()),
        (
            "currency_exponent".to_owned(),
            currency.exponent().to_string(),
        ),
    ];
    for (file, count) in rows {
        entries.push((format!("{}.rows", file.name()), count.to_string()));
    }

    let mut writer = writer(directory, ExportFile::Manifest)?;
    writer.write_record(["key", "value"])?;
    for (key, value) in &entries {
        writer.write_record([key.as_str(), value.as_str()])?;
    }
    writer.flush()?;

    Ok(entries.len())
}
