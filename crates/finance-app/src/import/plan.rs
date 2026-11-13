//! What an import would do, and what it did.
//!
//! [`ImportPlan`] is the answer the review screen shows before anything is
//! written: how many rows, how many duplicates, how many still need a decision,
//! and the in and out totals for the rows that would actually be posted
//! (FR-032, FR-033). [`ImportReport`] is the same shape after the commit, so the
//! screen can confirm what happened without re-reading the ledger.

use finance_core::Money;
use finance_import::OpeningBalance;

/// What a commit would write, given the decisions so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportPlan {
    pub(crate) rows: usize,
    pub(crate) duplicates: usize,
    pub(crate) to_post: usize,
    pub(crate) skipped: usize,
    pub(crate) undecided: usize,
    pub(crate) transfers: usize,
    pub(crate) total_in: Money,
    pub(crate) total_out: Money,
    pub(crate) opening: Option<OpeningBalance>,
    pub(crate) records_opening: bool,
}

impl ImportPlan {
    /// Rows that became transactions, before any decision.
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Rows that repeat the ledger or an earlier row in the file, whatever the
    /// user then decided.
    pub const fn duplicates(&self) -> usize {
        self.duplicates
    }

    /// Rows that will be written.
    pub const fn to_post(&self) -> usize {
        self.to_post
    }

    /// Rows that will be left out.
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    /// Rows still waiting for a category or a transfer.
    pub const fn undecided(&self) -> usize {
        self.undecided
    }

    /// Rows that will be written as transfers between the owner's accounts.
    pub const fn transfers(&self) -> usize {
        self.transfers
    }

    /// Money in over the rows that will be posted, excluding the opening
    /// balance.
    pub const fn total_in(&self) -> Money {
        self.total_in
    }

    /// Money out over the rows that will be posted, excluding the opening
    /// balance.
    pub const fn total_out(&self) -> Money {
        self.total_out
    }

    /// The opening balance the file carried, if any.
    pub const fn opening(&self) -> Option<&OpeningBalance> {
        self.opening.as_ref()
    }

    /// True when the commit will also write the opening balance.
    ///
    /// False when the file has none, or when the account already has one: a
    /// second opening balance would count the starting money twice.
    pub const fn records_opening(&self) -> bool {
        self.records_opening
    }

    /// True when every row has a decision, so a commit can run.
    pub const fn is_ready(&self) -> bool {
        self.undecided == 0
    }
}

/// What a commit wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportReport {
    posted: usize,
    skipped: usize,
    transfers: usize,
    opening: Option<Money>,
    total_in: Money,
    total_out: Money,
}

impl ImportReport {
    pub(crate) const fn new(
        posted: usize,
        skipped: usize,
        transfers: usize,
        opening: Option<Money>,
        total_in: Money,
        total_out: Money,
    ) -> Self {
        Self {
            posted,
            skipped,
            transfers,
            opening,
            total_in,
            total_out,
        }
    }

    /// Transactions written for statement rows, excluding the opening balance.
    pub const fn posted(&self) -> usize {
        self.posted
    }

    /// Rows left out.
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    /// Posted rows that are transfers between the owner's accounts.
    pub const fn transfers(&self) -> usize {
        self.transfers
    }

    /// The opening balance written, when the commit wrote one.
    pub const fn opening(&self) -> Option<Money> {
        self.opening
    }

    /// Money in over the posted rows.
    pub const fn total_in(&self) -> Money {
        self.total_in
    }

    /// Money out over the posted rows.
    pub const fn total_out(&self) -> Money {
        self.total_out
    }
}
