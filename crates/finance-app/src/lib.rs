//! Use cases for Finance, between the domain and whatever drives it.
//!
//! This crate answers "what does the user do next", not "what is true". It
//! combines [`finance_core`] with [`finance_import`], holds the state of a
//! half-finished action, and hands the result back as plain data. It performs no
//! I/O, reads no clock and generates no identifiers: the caller supplies the
//! ledger, the dates are the statement's own, and a commit takes the next
//! transaction id from a closure.
//!
//! Keeping that boundary here rather than in the UI is what makes a review screen
//! testable without a window, and what stops the UI from growing business rules
//! (ADR-0002).
//!
//! The first use case is the import review ([`import`]): a parsed statement, the
//! user's decision per row, and one atomic commit (FR-033 … FR-036).

#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]
#![deny(unsafe_op_in_unsafe_fn)]

mod error;
pub mod import;

pub use error::SessionError;
