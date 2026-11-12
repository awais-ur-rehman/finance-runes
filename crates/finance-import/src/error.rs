//! What can stop a parse, and what can spoil a single row.
//!
//! The split is deliberate. A structural failure stops the import because
//! nothing useful can be read without a header or decodable text. A bad row is
//! recorded and the rest of the file is still parsed, so one mistyped line does
//! not cost the user the other three hundred.
//!
//! Messages name what failed, never the value that failed. The value is in the
//! variant for the preview to display next to the line number; keeping it out of
//! `Display` means these messages can be logged without leaking an amount or a
//! payee (see `docs/engineering/data-privacy-and-security.md`).

use std::fmt;

use finance_core::Money;
use thiserror::Error;

/// A statement that cannot be read at all.
#[derive(Debug, Error)]
pub enum ImportError {
    /// The file holds no text.
    #[error("the file is empty")]
    Empty,

    /// The bytes are neither UTF-8 nor UTF-16.
    #[error("the file is not readable text: {detail}")]
    Encoding {
        /// What the decoder found, without echoing any content.
        detail: String,
    },

    /// No row looks like the profile's header.
    #[error(
        "no row in the first {limit} matches the {profile} profile; expected the columns {expected}"
    )]
    MissingHeader {
        /// The profile's name.
        profile: String,
        /// The columns the profile needs, joined for display.
        expected: String,
        /// How many rows were searched.
        limit: usize,
    },

    /// The header exists but one profile column is not in it.
    #[error("the header has no {field} column")]
    MissingColumn {
        /// The column the profile names.
        field: String,
    },

    /// The header names the same column twice, so a row's value is ambiguous.
    #[error("the header has more than one {field} column")]
    DuplicateColumn {
        /// The column the profile names.
        field: String,
    },

    /// The reader itself failed, which should not happen after decoding.
    #[error("the file could not be read as CSV: {detail}")]
    Malformed {
        /// What the reader reported.
        detail: String,
    },

    /// The statement's totals do not fit in a 64-bit count of minor units.
    #[error("the statement totals are too large to represent")]
    Overflow,
}

/// One row that could not become a transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowIssue {
    line: usize,
    problem: RowProblem,
}

impl RowIssue {
    pub(crate) const fn new(line: usize, problem: RowProblem) -> Self {
        Self { line, problem }
    }

    /// The 1-based line in the file, counting the header, so the preview can
    /// point at the exact row the user has to fix.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// What is wrong with the row.
    pub const fn problem(&self) -> &RowProblem {
        &self.problem
    }
}

impl fmt::Display for RowIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.problem)
    }
}

/// Why a row could not be used.
///
/// Each variant carries the offending value so the preview can show it beside
/// the line number; `Display` deliberately does not print it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowProblem {
    /// The row has a different number of columns than the header.
    WrongColumnCount {
        /// Columns in the header.
        expected: usize,
        /// Columns in this row.
        found: usize,
    },

    /// The date does not match the profile's format.
    Date {
        /// The text as it appeared in the file.
        value: String,
    },

    /// A direction column does not hold a number.
    Amount {
        /// The profile's name for the column.
        column: String,
        /// The text as it appeared in the file.
        value: String,
    },

    /// A one-direction column holds a negative number, so its direction is
    /// ambiguous.
    Negative {
        /// The profile's name for the column.
        column: String,
        /// The text as it appeared in the file.
        value: String,
    },

    /// Both the debit and the credit column carry an amount.
    BothDirections {
        /// The debit cell.
        debit: String,
        /// The credit cell.
        credit: String,
    },

    /// The row moves no money and is not an opening balance.
    NoAmount,

    /// The balance cell does not hold a number.
    Balance {
        /// The text as it appeared in the file.
        value: String,
    },

    /// The description is empty, so the row cannot be identified or reviewed.
    NoDescription,

    /// An opening balance row has no balance to take the opening amount from.
    OpeningBalanceWithoutBalance,

    /// A second opening balance row was found.
    RepeatedOpeningBalance,

    /// The opening balance is zero or negative, which the ledger cannot record
    /// as an opening balance yet.
    OpeningBalanceNotPositive {
        /// The amount as parsed.
        amount: Money,
    },
}

impl fmt::Display for RowProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongColumnCount { expected, found } => {
                write!(formatter, "has {found} columns, the header has {expected}")
            }
            Self::Date { .. } => {
                formatter.write_str("the date does not match the profile's format")
            }
            Self::Amount { column, .. } => {
                write!(formatter, "the {column} amount is not a number")
            }
            Self::Negative { column, .. } => write!(
                formatter,
                "the {column} amount is negative, but direction comes from the column"
            ),
            Self::BothDirections { .. } => {
                formatter.write_str("both the debit and the credit column carry an amount")
            }
            Self::NoAmount => {
                formatter.write_str("the row moves no money and is not an opening balance")
            }
            Self::Balance { .. } => formatter.write_str("the balance is not a number"),
            Self::NoDescription => formatter.write_str("the description is empty"),
            Self::OpeningBalanceWithoutBalance => {
                formatter.write_str("an opening balance row without a balance")
            }
            Self::RepeatedOpeningBalance => formatter.write_str("a second opening balance row"),
            Self::OpeningBalanceNotPositive { .. } => formatter
                .write_str("the opening balance is zero or negative, which cannot be recorded yet"),
        }
    }
}
