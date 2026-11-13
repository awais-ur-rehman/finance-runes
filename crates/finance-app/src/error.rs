//! What a use case can refuse, and what it passes through from the domain.
//!
//! Every variant names something the caller can act on: a row index a screen can
//! highlight, a count it can report, or a domain error it can show unchanged.
//! `Display` never prints an amount or a payee, so these messages are safe to
//! log (see `docs/engineering/data-privacy-and-security.md`).

use finance_core::{AccountId, Currency, LedgerError, TransactionError};
use thiserror::Error;

/// Something stopped a use case before it changed anything.
#[derive(Debug, Error)]
pub enum SessionError {
    /// The row index is outside the review list.
    #[error("row {index} is not part of this import")]
    UnknownRow {
        /// The index the caller asked for.
        index: usize,
    },

    /// The destination account is not in the ledger.
    #[error("the destination account is not in this ledger")]
    UnknownAccount(AccountId),

    /// The destination account is archived, so new rows must not be filed under
    /// it. History keeps the rows it already has.
    #[error("the destination account is archived")]
    ArchivedAccount(AccountId),

    /// The statement and the ledger disagree on currency.
    #[error("the statement is in {statement} but the ledger is in {ledger}")]
    CurrencyMismatch {
        /// The ledger's currency.
        ledger: Currency,
        /// The statement's currency.
        statement: Currency,
    },

    /// A transfer points back at the account being imported, which would move
    /// money to where it already is.
    #[error("a transfer cannot use the account being imported as its other side")]
    SameAccount(AccountId),

    /// Some rows have neither a category nor a transfer yet.
    #[error("{rows} rows still need a category or a transfer")]
    Undecided {
        /// How many rows are waiting for a decision.
        rows: usize,
    },

    /// The session has already written its rows. A second commit would import
    /// the same statement twice.
    #[error("this import has already been committed")]
    AlreadyCommitted,

    /// A total does not fit in a 64-bit count of minor units.
    #[error("the import totals are too large to represent")]
    Overflow,

    /// A row could not be turned into a balanced transaction.
    #[error(transparent)]
    Transaction(#[from] TransactionError),

    /// The ledger refused the batch. Nothing was written.
    #[error(transparent)]
    Ledger(#[from] LedgerError),
}
