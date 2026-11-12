//! Reading a bank statement into rows, without touching the filesystem.
//!
//! The caller reads bytes and passes them in; this crate decodes, parses and
//! checks them. Nothing here reads a clock, opens a file or writes to storage,
//! which is what makes the whole importer testable from a string literal.
//!
//! The rule that shapes the API is that **one bad row must not cost the user the
//! rest of the file**. A missing header or undecodable bytes stop the import,
//! because nothing can be read without them. A row with a broken date, a broken
//! amount or a broken balance is recorded as a [`RowIssue`] with its line number,
//! and parsing continues. That is what the preview in FR-032 needs.
//!
//! The parser does not decide what a row *means*. It reports the flow of money,
//! a merchant hint and a transfer hint; assigning a category or a counter-account
//! belongs to the review step, because it needs the user's ledger and their
//! judgement (FR-033).
//!
//! The shape being parsed is recorded in
//! `docs/requirements/csv-import-profile.md`, including the synthetic fixture the
//! tests use.

#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]
#![deny(unsafe_op_in_unsafe_fn)]

mod amount;
mod balance;
mod date;
mod duplicate;
mod error;
mod merchant;
mod preview;
mod profile;
mod reader;
mod statement;
mod text;

pub use amount::parse_amount;
pub use balance::BalanceBreak;
pub use date::DateFormat;
pub use date::parse_date;
pub use duplicate::Duplicate;
pub use duplicate::DuplicateKey;
pub use duplicate::ReferenceKind;
pub use error::ImportError;
pub use error::RowIssue;
pub use error::RowProblem;
pub use merchant::TransferHint;
pub use merchant::merchant;
pub use merchant::transfer_hint;
pub use preview::Preview;
pub use profile::Direction;
pub use profile::Profile;
pub use reader::MAX_PREAMBLE;
pub use reader::parse;
pub use statement::Flow;
pub use statement::OpeningBalance;
pub use statement::Statement;
pub use statement::StatementRow;
