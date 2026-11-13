//! Remembered choices: what makes a later row preselect a category or a transfer.
//!
//! FR-033 asks the app to remember "merchant → category" and "pattern → transfer"
//! and apply both to later rows. The key is a value the importer already
//! derived, never the whole description: a description can carry an account
//! number or a reference, and a rule keyed on one of those would both stop
//! matching the next transfer and copy the number into every later row.
//!
//! A rule is matched against the *derived* value, so a card row whose city or
//! formatting changed still matches its merchant rule.

use finance_core::AccountId;
use finance_import::{StatementRow, TransferHint};

use crate::import::decision::Entry;

/// A choice the session applies to later rows that look the same.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// A merchant name maps to a category, or to another account.
    Merchant {
        /// The merchant name as the importer read it, e.g. `SAMPLE CAFE`.
        merchant: String,
        /// What a matching row becomes.
        entry: Entry,
    },

    /// A kind of own-account movement maps to the other side.
    ///
    /// Keyed on the hint rather than on text, so every cash withdrawal is the
    /// same rule and every peer transfer from the same counterparty is another.
    Transfer {
        /// The pattern the importer recognised.
        hint: TransferHint,
        /// The other account the money moved to or from.
        account: AccountId,
    },
}

impl Rule {
    /// The rule a row teaches when the user files it, if it teaches one.
    ///
    /// A transfer is remembered as its pattern. Anything else is remembered by
    /// its merchant name, and a row with neither teaches nothing: guessing a
    /// pattern from free text is how a single correction turns into forty wrong
    /// ones.
    #[must_use]
    pub fn learned_from(row: &StatementRow, entry: Entry) -> Option<Self> {
        if let (Entry::Transfer(account), Some(hint)) = (entry, row.transfer_hint()) {
            return Some(Self::Transfer { hint, account });
        }

        row.merchant()
            .map(|merchant| Self::Merchant { merchant, entry })
    }

    /// The entry this rule gives a row, when it matches.
    ///
    /// Transfer hints are checked by the caller first, so a merchant rule can
    /// never override a recognised own-account movement.
    #[must_use]
    pub fn apply(&self, row: &StatementRow) -> Option<Entry> {
        match self {
            Self::Merchant { merchant, entry } => row
                .merchant()
                .filter(|found| same_merchant(found, merchant))
                .map(|_| *entry),
            Self::Transfer { hint, account } => row
                .transfer_hint()
                .filter(|found| found == hint)
                .map(|_| Entry::Transfer(*account)),
        }
    }

    /// True when both rules are about the same thing, so the newer one replaces
    /// the older.
    pub(crate) fn same_subject(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Merchant { merchant, .. },
                Self::Merchant {
                    merchant: other, ..
                },
            ) => same_merchant(merchant, other),
            (Self::Transfer { hint, .. }, Self::Transfer { hint: other, .. }) => hint == other,
            _ => false,
        }
    }
}

/// Compares merchant names the way the importer reads them: trimmed, ignoring
/// ASCII case. Spacing is already canonical, because the hint is built from
/// whitespace-separated tokens.
fn same_merchant(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}
