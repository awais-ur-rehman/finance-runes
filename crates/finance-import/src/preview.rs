//! The aggregate the preview screen shows before anything is committed.
//!
//! FR-032 asks for row count, date range, total in, total out, duplicates,
//! unparseable rows with line numbers, and the first break in the running
//! balance with both balances. All of it is derived once, at parse time, so the
//! preview cannot disagree with the rows the review screen then edits.

use finance_core::Money;
use time::Date;

use crate::balance::BalanceBreak;
use crate::duplicate::Duplicate;
use crate::error::RowIssue;
use crate::statement::{OpeningBalance, Statement};

/// What the user sees before committing an import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    rows: usize,
    transactions: usize,
    suspected_transfers: usize,
    opening_balance: Option<OpeningBalance>,
    date_range: Option<(Date, Date)>,
    total_in: Money,
    total_out: Money,
    final_balance: Option<Money>,
    issues: Vec<RowIssue>,
    balance_breaks: Vec<BalanceBreak>,
    duplicates: Vec<Duplicate>,
}

impl Preview {
    pub(crate) fn new(statement: &Statement) -> Self {
        let final_balance = statement
            .rows()
            .iter()
            .rev()
            .find_map(|row| row.balance())
            .or_else(|| statement.opening_balance().map(OpeningBalance::amount));

        Self {
            rows: statement.data_rows(),
            transactions: statement.rows().len(),
            suspected_transfers: statement
                .rows()
                .iter()
                .filter(|row| row.transfer_hint().is_some())
                .count(),
            opening_balance: statement.opening_balance().cloned(),
            date_range: statement.date_range(),
            total_in: statement.total_in(),
            total_out: statement.total_out(),
            final_balance,
            issues: statement.issues().to_vec(),
            balance_breaks: statement.balance_breaks().to_vec(),
            duplicates: statement.duplicates().to_vec(),
        }
    }

    /// Non-empty rows after the header.
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Rows that can become transactions.
    pub const fn transactions(&self) -> usize {
        self.transactions
    }

    /// Rows whose description suggests a transfer between the owner's accounts.
    pub const fn suspected_transfers(&self) -> usize {
        self.suspected_transfers
    }

    /// The opening balance the file carried, if any.
    pub const fn opening_balance(&self) -> Option<&OpeningBalance> {
        self.opening_balance.as_ref()
    }

    /// The earliest and latest transaction date.
    pub const fn date_range(&self) -> Option<(Date, Date)> {
        self.date_range
    }

    /// Total money in, excluding the opening balance.
    pub const fn total_in(&self) -> Money {
        self.total_in
    }

    /// Total money out, excluding the opening balance.
    pub const fn total_out(&self) -> Money {
        self.total_out
    }

    /// The balance after the last row, or the opening balance when the file has
    /// no rows.
    pub const fn final_balance(&self) -> Option<Money> {
        self.final_balance
    }

    /// Rows that could not be used.
    pub fn issues(&self) -> &[RowIssue] {
        &self.issues
    }

    /// Places where the running balance does not add up.
    pub fn balance_breaks(&self) -> &[BalanceBreak] {
        &self.balance_breaks
    }

    /// Rows that repeat a key already used in the file.
    pub fn duplicates(&self) -> &[Duplicate] {
        &self.duplicates
    }

    /// True when nothing needs the user's attention.
    ///
    /// The user can still commit an unclean preview: a break might be a known
    /// gap in the statement, and a duplicate might be a genuine second purchase.
    /// The flag is a warning, not a gate.
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty() && self.balance_breaks.is_empty() && self.duplicates.is_empty()
    }
}

#[cfg(test)]
mod tests;
