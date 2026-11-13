//! The domain model for Finance: money, accounts, categories and the ledger.
//!
//! This crate is deliberately pure. It performs no I/O, reads no clock, generates
//! no randomness and knows nothing about storage or the UI. Callers pass in dates
//! and identifiers, which keeps every rule deterministic and testable in
//! milliseconds.
//!
//! The rules implemented here come from `docs/requirements/` and the architecture
//! decisions in `docs/architecture/decisions/`. In particular:
//!
//! - money is an integer count of minor units plus a currency ([ADR-0003]);
//! - the ledger is double-entry behind a single-entry UI ([ADR-0005]);
//! - balances are derived from postings, never stored as truth.
//!
//! [ADR-0003]: https://github.com/awais-ur-rehman/finance-runes/blob/main/docs/architecture/decisions/0003-money-and-currency.md
//! [ADR-0005]: https://github.com/awais-ur-rehman/finance-runes/blob/main/docs/architecture/decisions/0005-ledger-model.md

#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]
#![deny(unsafe_op_in_unsafe_fn)]

mod account;
mod budget;
mod category;
mod error;
mod id;
mod ledger;
pub mod money;
mod transaction;

pub use account::Account;
pub use account::AccountKind;
pub use budget::Budget;
pub use budget::BudgetLine;
pub use budget::BudgetReport;
pub use budget::BudgetState;
pub use budget::DEFAULT_NEAR_LIMIT_PERCENT;
pub use category::Category;
pub use category::CategoryKind;
pub use error::LedgerError;
pub use error::MoneyError;
pub use error::TransactionError;
pub use id::AccountId;
pub use id::CategoryId;
pub use id::TransactionId;
pub use ledger::Ledger;
pub use ledger::MonthSummary;
pub use ledger::YearMonth;
pub use money::Currency;
pub use money::Money;
pub use money::Rounding;
pub use transaction::Posting;
pub use transaction::PostingTarget;
pub use transaction::Transaction;
pub use transaction::TransactionKind;
pub use transaction::TransactionSource;
