//! Typed errors. Each variant names a failure the caller can act on.

use crate::money::Currency;
use crate::{AccountId, CategoryId, CategoryKind, TransactionId, TransactionKind};

/// Something went wrong while building or combining an amount.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MoneyError {
    /// The two amounts are in different currencies and cannot be combined.
    #[error("currency mismatch: {left} and {right}")]
    CurrencyMismatch {
        /// Currency of the left operand.
        left: Currency,
        /// Currency of the right operand.
        right: Currency,
    },
    /// The result does not fit in an `i64` of minor units.
    #[error("amount overflow")]
    Overflow,
    /// A divisor was zero.
    #[error("division by zero")]
    DivisionByZero,
    /// A currency code was not three uppercase ASCII letters.
    #[error("invalid currency code: {code}")]
    InvalidCurrency {
        /// The rejected code.
        code: String,
    },
    /// A currency exponent was outside the supported range.
    #[error("currency exponent {exponent} is outside 0..=4")]
    InvalidExponent {
        /// The rejected exponent.
        exponent: u8,
    },
    /// The input was not a valid decimal amount.
    #[error("invalid amount: {input}")]
    InvalidAmount {
        /// The rejected input.
        input: String,
    },
    /// The input carried more decimal places than the currency allows.
    #[error("{currency} allows {max} decimal places, got {found}")]
    TooManyDecimals {
        /// The currency whose exponent was exceeded.
        currency: Currency,
        /// Maximum allowed decimal places.
        max: u8,
        /// Decimal places actually supplied.
        found: usize,
    },
    /// An allocation was requested with no usable weights.
    #[error("allocation needs at least one non-zero weight")]
    EmptyAllocation,
}

/// A transaction could not be built from its postings.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransactionError {
    /// Fewer than two postings were supplied.
    #[error("a transaction needs at least two postings")]
    TooFewPostings,
    /// The postings are not all in the same currency.
    #[error("all postings in a transaction must share one currency")]
    CurrencyMismatch,
    /// The postings do not net to zero.
    #[error("postings must sum to zero, but they sum to {minor} minor units")]
    Unbalanced {
        /// The net total, in minor units.
        minor: i128,
    },
    /// A convenience constructor was given a zero or negative amount.
    #[error("amount must be positive")]
    NonPositiveAmount,
    /// The positive postings do not fit in an `i64` of minor units.
    #[error("transaction amount overflow")]
    Overflow,
}

/// A transaction could not be added to a ledger.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LedgerError {
    /// Money arithmetic failed while computing a total.
    #[error(transparent)]
    Money(#[from] MoneyError),
    /// A transaction could not be constructed.
    #[error(transparent)]
    Transaction(#[from] TransactionError),
    /// An account with this identifier is already present.
    #[error("account {0} already exists")]
    DuplicateAccount(AccountId),
    /// A category with this identifier is already present.
    #[error("category {0} already exists")]
    DuplicateCategory(CategoryId),
    /// A transaction with this identifier is already present.
    #[error("transaction {0} already exists")]
    DuplicateTransaction(TransactionId),
    /// A posting referenced an account the ledger does not know.
    #[error("unknown account {0}")]
    UnknownAccount(AccountId),
    /// A posting referenced a category the ledger does not know.
    #[error("unknown category {0}")]
    UnknownCategory(CategoryId),
    /// A category's parent is not in the ledger.
    #[error("unknown parent category {0}")]
    UnknownParent(CategoryId),
    /// Categories may only nest two levels deep.
    #[error("categories may nest at most two levels")]
    CategoryTooDeep,
    /// A category's kind does not match the transaction's kind.
    #[error("expected a {expected} category, but {found} was given")]
    CategoryKindMismatch {
        /// The kind the transaction requires.
        expected: CategoryKind,
        /// The kind the category actually has.
        found: CategoryKind,
    },
    /// An account's currency does not match the ledger's currency.
    #[error("account currency {account} does not match ledger currency {ledger}")]
    AccountCurrencyMismatch {
        /// The account's currency.
        account: Currency,
        /// The ledger's currency.
        ledger: Currency,
    },
    /// The transaction's postings do not have the shape its kind requires.
    #[error("invalid shape for a {kind} transaction")]
    InvalidShape {
        /// The transaction kind whose rules were broken.
        kind: TransactionKind,
    },
    /// A posting appeared twice for the same account.
    #[error("a transaction cannot post twice to the same account")]
    DuplicateAccountPosting,
}
