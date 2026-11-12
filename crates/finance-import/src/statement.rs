//! A parsed statement: the transactions, the opening balance, and everything
//! that is wrong with them.
//!
//! This is the importer's working result, not the ledger. Nothing here has been
//! categorised, because that needs the user's categories and their confirmation
//! (FR-033). A statement can be previewed and thrown away without touching
//! storage.

use finance_core::{Currency, Money};
use time::Date;

use crate::balance::{self, BalanceBreak};
use crate::duplicate::{self, Duplicate, DuplicateKey};
use crate::error::{ImportError, RowIssue};
use crate::merchant::{self, TransferHint};
use crate::preview::Preview;

/// Which way money moved.
///
/// The amount is always positive; the direction is the variant. A signed amount
/// with a flag beside it is the shape that lets `-0` and double negatives exist,
/// so this type does not allow one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flow {
    incoming: bool,
    amount: Money,
}

impl Flow {
    /// Money arriving, or `None` when the amount is not positive.
    pub fn incoming(amount: Money) -> Option<Self> {
        if amount.is_positive() {
            Some(Self {
                incoming: true,
                amount,
            })
        } else {
            None
        }
    }

    /// Money leaving, or `None` when the amount is not positive.
    pub fn outgoing(amount: Money) -> Option<Self> {
        if amount.is_positive() {
            Some(Self {
                incoming: false,
                amount,
            })
        } else {
            None
        }
    }

    /// The amount moved, always positive.
    pub const fn amount(self) -> Money {
        self.amount
    }

    /// True when money arrived.
    pub const fn is_incoming(self) -> bool {
        self.incoming
    }

    /// True when money left.
    pub const fn is_outgoing(self) -> bool {
        !self.incoming
    }

    /// The amount as a signed number: positive in, negative out.
    ///
    /// Used where two flows have to compare as one value, such as a duplicate
    /// key. The amount is positive by construction, so negating it cannot
    /// overflow.
    pub fn signed(self) -> Money {
        let minor = if self.incoming {
            self.amount.minor()
        } else {
            -self.amount.minor()
        };
        Money::from_minor(minor, self.amount.currency())
    }
}

/// The balance an account started with, taken from the marker row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningBalance {
    line: usize,
    date: Date,
    amount: Money,
}

impl OpeningBalance {
    pub(crate) const fn new(line: usize, date: Date, amount: Money) -> Self {
        Self { line, date, amount }
    }

    /// The line the marker row was on.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// The date on the marker row.
    pub const fn date(&self) -> Date {
        self.date
    }

    /// The starting balance.
    pub const fn amount(&self) -> Money {
        self.amount
    }
}

/// One row that became a transaction candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementRow {
    line: usize,
    date: Date,
    description: String,
    instrument: Option<String>,
    flow: Flow,
    balance: Option<Money>,
}

impl StatementRow {
    pub(crate) fn new(
        line: usize,
        date: Date,
        description: String,
        instrument: Option<String>,
        flow: Flow,
        balance: Option<Money>,
    ) -> Self {
        Self {
            line,
            date,
            description,
            instrument,
            flow,
            balance,
        }
    }

    /// The 1-based line in the file.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// The date the bank posted the row.
    pub const fn date(&self) -> Date {
        self.date
    }

    /// The description exactly as the bank wrote it, trimmed.
    ///
    /// Kept whole because it is what the user recognises in the register, and
    /// because the merchant hint is only a suggestion derived from it.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The instrument number, when the bank supplied one.
    pub fn instrument(&self) -> Option<&str> {
        self.instrument.as_deref()
    }

    /// Which way the money moved and how much.
    pub const fn flow(&self) -> Flow {
        self.flow
    }

    /// The account balance after this row, when the file provides one.
    pub const fn balance(&self) -> Option<Money> {
        self.balance
    }

    /// The shortened merchant name, when the description carries one.
    pub fn merchant(&self) -> Option<String> {
        merchant::merchant(&self.description)
    }

    /// A guess that this row moves money between the owner's own accounts.
    pub fn transfer_hint(&self) -> Option<TransferHint> {
        merchant::transfer_hint(&self.description)
    }

    /// The key used to recognise this row if it is imported twice (FR-035).
    pub fn duplicate_key(&self) -> DuplicateKey {
        DuplicateKey::for_row(self)
    }
}

/// A parsed statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    currency: Currency,
    header_line: usize,
    data_rows: usize,
    rows: Vec<StatementRow>,
    opening: Option<OpeningBalance>,
    issues: Vec<RowIssue>,
    total_in: Money,
    total_out: Money,
    balance_breaks: Vec<BalanceBreak>,
    duplicates: Vec<Duplicate>,
    date_range: Option<(Date, Date)>,
}

impl Statement {
    /// Builds the derived facts once, so the preview and the review screen
    /// cannot disagree about totals or breaks.
    pub(crate) fn assemble(
        currency: Currency,
        header_line: usize,
        data_rows: usize,
        rows: Vec<StatementRow>,
        opening: Option<OpeningBalance>,
        issues: Vec<RowIssue>,
    ) -> Result<Self, ImportError> {
        let (total_in, total_out) = totals(&rows, currency)?;
        let gaps: Vec<usize> = issues.iter().map(RowIssue::line).collect();
        let balance_breaks = balance::check(opening.as_ref(), &rows, &gaps);
        let duplicates = duplicate::within(&rows);
        let date_range = date_range(&rows);

        Ok(Self {
            currency,
            header_line,
            data_rows,
            rows,
            opening,
            issues,
            total_in,
            total_out,
            balance_breaks,
            duplicates,
            date_range,
        })
    }

    /// The currency the statement was parsed into.
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// The 1-based line of the header row, which is not always line 1.
    pub const fn header_line(&self) -> usize {
        self.header_line
    }

    /// How many non-empty rows follow the header.
    ///
    /// Every one of them is in exactly one of [`Statement::rows`],
    /// [`Statement::opening_balance`] or [`Statement::issues`].
    pub const fn data_rows(&self) -> usize {
        self.data_rows
    }

    /// The rows that can become transactions.
    pub fn rows(&self) -> &[StatementRow] {
        &self.rows
    }

    /// The account's starting balance, when the file carried one.
    pub const fn opening_balance(&self) -> Option<&OpeningBalance> {
        self.opening.as_ref()
    }

    /// Rows that could not be used, each with its line number.
    pub fn issues(&self) -> &[RowIssue] {
        &self.issues
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

    /// Places where the file's running balance stops adding up.
    pub fn balance_breaks(&self) -> &[BalanceBreak] {
        &self.balance_breaks
    }

    /// The first break, which is the one the preview names (FR-032).
    pub fn first_balance_break(&self) -> Option<&BalanceBreak> {
        self.balance_breaks.first()
    }

    /// Rows that repeat a key already used in the file.
    pub fn duplicates(&self) -> &[Duplicate] {
        &self.duplicates
    }

    /// The aggregate the preview screen renders (FR-032).
    pub fn preview(&self) -> Preview {
        Preview::new(self)
    }
}

fn totals(rows: &[StatementRow], currency: Currency) -> Result<(Money, Money), ImportError> {
    let mut incoming: i128 = 0;
    let mut outgoing: i128 = 0;
    for row in rows {
        let amount = i128::from(row.flow().amount().minor());
        if row.flow().is_incoming() {
            incoming += amount;
        } else {
            outgoing += amount;
        }
    }

    let incoming = i64::try_from(incoming).map_err(|_| ImportError::Overflow)?;
    let outgoing = i64::try_from(outgoing).map_err(|_| ImportError::Overflow)?;
    Ok((
        Money::from_minor(incoming, currency),
        Money::from_minor(outgoing, currency),
    ))
}

fn date_range(rows: &[StatementRow]) -> Option<(Date, Date)> {
    let mut earliest = None;
    let mut latest = None;
    for row in rows {
        earliest = Some(earliest.map_or(row.date, |date: Date| date.min(row.date)));
        latest = Some(latest.map_or(row.date, |date: Date| date.max(row.date)));
    }
    earliest.zip(latest)
}

#[cfg(test)]
mod tests;
