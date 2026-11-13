//! What the user decided about one row, and what that turns into.
//!
//! Direction is deliberately not part of [`Entry`]. It comes from the row's
//! [`Flow`](finance_import::Flow), so an entry can never contradict the
//! statement: money out becomes an expense or a transfer out, money in becomes
//! income or a transfer in. That removes a whole class of "the balance went the
//! wrong way" bug from the review screen.

use finance_core::{AccountId, CategoryId};

/// What a reviewed row becomes in the ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// File the row against a category: an expense when money left, income when
    /// money arrived.
    Category(CategoryId),

    /// Move the money between the imported account and another of the owner's
    /// accounts. A cash withdrawal is this with the cash account as the other
    /// side; a peer transfer in is this with the account it came from.
    Transfer(AccountId),
}

/// Why a row repeats something already recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicateState {
    /// A transaction with the same date, amount and description is already in
    /// the ledger for this account (FR-035).
    InLedger,

    /// An earlier row in the same file has the same key.
    InFile {
        /// The line of the earlier row.
        first_line: usize,
    },
}

/// Why a row will not be imported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// Left out because it repeats something already present.
    Duplicate(DuplicateState),

    /// The user chose not to import a row that is not a duplicate.
    Chosen,
}

/// The user's decision about a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// No decision yet. The row is neither imported nor skipped, and a commit
    /// refuses to run until every row has one.
    Undecided,

    /// Leave the row out.
    Skip(SkipReason),

    /// Write the row to the ledger.
    Post(Entry),
}
