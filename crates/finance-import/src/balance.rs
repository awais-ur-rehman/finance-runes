//! Checking the running balance a statement provides.
//!
//! This is the cheapest correctness check the importer will ever get (FR-032).
//! If `previous - out + in == balance` holds for every row, the file was read
//! the way the bank wrote it. A break means a misparsed row, a skipped row or a
//! row that is already in the ledger.
//!
//! After a break the check continues from the balance the file states, not from
//! the one arithmetic expected. One bad row should produce one break, not a
//! cascade of them.

use finance_core::Money;

use crate::statement::{Flow, OpeningBalance, StatementRow};

/// A row whose stated balance does not follow from the previous one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalanceBreak {
    line: usize,
    expected: Money,
    found: Money,
}

impl BalanceBreak {
    /// The line whose balance does not add up.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// The balance the previous row and this row's amount imply.
    pub const fn expected(&self) -> Money {
        self.expected
    }

    /// The balance the file states.
    pub const fn found(&self) -> Money {
        self.found
    }
}

/// Returns every row where the running balance breaks.
///
/// `gaps` are the lines of rows that could not be parsed. A gap ends the chain:
/// the next parsed row anchors the check with the balance the file states,
/// because a dropped row makes the arithmetic meaningless rather than wrong.
/// That way one bad row produces one issue instead of a break at every row after
/// it.
///
/// Verification also stops at the first parsed row whose balance could not be
/// read, for the same reason.
pub(crate) fn check(
    opening: Option<&OpeningBalance>,
    rows: &[StatementRow],
    gaps: &[usize],
) -> Vec<BalanceBreak> {
    let mut previous = opening.map(OpeningBalance::amount);
    let mut gaps = gaps.iter().copied().peekable();
    let mut breaks = Vec::new();

    for row in rows {
        while gaps.peek().is_some_and(|line| *line < row.line()) {
            gaps.next();
            previous = None;
        }

        let Some(found) = row.balance() else {
            break;
        };

        if let Some(previous) = previous {
            let Some(expected) = expected_balance(previous, row.flow()) else {
                break;
            };
            if expected != found {
                breaks.push(BalanceBreak {
                    line: row.line(),
                    expected,
                    found,
                });
            }
        }

        previous = Some(found);
    }

    breaks
}

/// `previous - out + in`, or `None` when the result does not fit.
fn expected_balance(previous: Money, flow: Flow) -> Option<Money> {
    let amount = i128::from(flow.amount().minor());
    let value = i128::from(previous.minor()) + if flow.is_incoming() { amount } else { -amount };
    i64::try_from(value)
        .ok()
        .map(|minor| Money::from_minor(minor, previous.currency()))
}

#[cfg(test)]
mod tests;
