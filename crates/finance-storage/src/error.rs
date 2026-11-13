//! What can go wrong while reading or writing a dataset.

use std::path::PathBuf;

use finance_core::{Currency, LedgerError, TransactionError, TransactionId};
use thiserror::Error;

/// A failure while reading or writing a dataset.
///
/// The distinction that matters to the caller is between a database that could
/// not be used (`Database`) and a database that opened but holds something this
/// build does not understand (`Corrupt`, `UnsupportedSchema`). The second kind
/// must never be silently repaired: the data is the user's only copy.
#[derive(Debug, Error)]
pub enum StoreError {
    /// SQLite itself failed.
    #[error("the dataset could not be read or written: {0}")]
    Database(#[from] rusqlite::Error),

    /// The dataset holds another currency than the one the store was opened for.
    #[error("the dataset is in {stored}, but this store was opened for {expected}")]
    CurrencyMismatch {
        /// Currency recorded in the dataset.
        stored: Currency,
        /// Currency the caller asked for.
        expected: Currency,
    },

    /// The ledger handed to [`save`](crate::Store::save) is in another currency.
    #[error("the ledger is in {ledger}, but this dataset is in {store}")]
    LedgerCurrency {
        /// Currency of the ledger being saved.
        ledger: Currency,
        /// Currency of the dataset.
        store: Currency,
    },

    /// The dataset was written by a newer build.
    #[error("the file uses schema version {found}, but this build supports up to {supported}")]
    UnsupportedSchema {
        /// Version found on disk.
        found: u32,
        /// Highest version this build can migrate to.
        supported: u32,
    },

    /// A backup failed its integrity check.
    #[error("the backup failed its integrity check and was not restored: {problems:?}")]
    Integrity {
        /// One line per problem, as SQLite reported it.
        problems: Vec<String>,
    },

    /// The target of a backup already exists.
    #[error("a file already exists at {}", .path.display())]
    BackupExists {
        /// The path that is already taken.
        path: PathBuf,
    },

    /// The directory an export would write to already exists.
    #[error("a directory already exists at {}", .path.display())]
    ExportExists {
        /// The path that is already taken.
        path: PathBuf,
    },

    /// A CSV file of an export could not be written.
    #[error("the export could not be written: {0}")]
    Csv(#[from] csv::Error),

    /// A path could not be used as text.
    #[error("the path {} cannot be used as text", .0.display())]
    InvalidPath(PathBuf),

    /// A file operation outside SQLite failed.
    #[error("the file could not be read or written: {0}")]
    Io(#[from] std::io::Error),

    /// A row exists but cannot be interpreted.
    #[error("the {table} table holds a corrupt row: {detail}")]
    Corrupt {
        /// Table the row came from.
        table: &'static str,
        /// What was wrong with it.
        detail: String,
    },

    /// The domain rejected data read from disk.
    #[error("the domain rejected the stored data: {0}")]
    Ledger(#[from] LedgerError),

    /// Stored postings do not form a valid transaction.
    #[error("the stored transaction is not valid: {0}")]
    Transaction(#[from] TransactionError),

    /// The identifier does not exist in the dataset.
    #[error("no transaction with id {0} is stored")]
    UnknownTransaction(TransactionId),
}
