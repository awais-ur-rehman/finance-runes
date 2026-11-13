//! The import review: what each parsed row becomes, and one atomic commit.
//!
//! The importer answers "what is in the file". This module answers "what should
//! the ledger do about it". It keeps the statement, a decision per row and the
//! choices the user has taught it, so the review screen can render a row, change
//! one, and ask what the result would be before writing anything.
//!
//! Three rules shape the state:
//!
//! - **A duplicate is skipped by default** (FR-035). The user can post it anyway;
//!   the default is not a gate.
//! - **An undecided row is never guessed at.** Committing with one left fails
//!   instead of filing it under a category the user did not choose.
//! - **A commit is all or nothing** (FR-034). Rows are built first, then handed
//!   to [`Ledger::apply`](finance_core::Ledger::apply) with the rules the review
//!   taught, so a failure half way through leaves neither a partial import nor a
//!   half-remembered rule.

mod decision;
mod plan;
mod rule;
mod session;
#[cfg(test)]
mod tests;

pub use decision::Decision;
pub use decision::DuplicateState;
pub use decision::Entry;
pub use decision::SkipReason;
pub use plan::ImportPlan;
pub use plan::ImportReport;
pub use session::ImportSession;
pub use session::ReviewRow;
