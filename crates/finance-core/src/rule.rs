//! Remembered classification rules: what the user taught the app about their
//! own money.
//!
//! FR-033 asks the app to remember "merchant → category" and "pattern →
//! transfer" and apply both to later rows. A rule is dataset state, not a
//! setting: it is saved with the ledger, so it survives a restart and travels
//! with a backup.
//!
//! A key is always a value the importer derived, never a raw description. A
//! description can carry an account number or a reference, and a rule keyed on
//! one of those would stop matching the next transfer and copy the number into
//! every later row.

use crate::{AccountId, CategoryId};

/// The kind of own-account movement a transfer rule is about.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferPattern {
    /// A cash withdrawal: the money is still the owner's, now in cash.
    CashWithdrawal,

    /// A transfer whose counterparty looks like the account holder.
    PeerTransfer {
        /// The counterparty, when it can be read out of the description.
        ///
        /// Canonical: trimmed, ASCII upper-case, and `None` when the description
        /// named nobody. Use [`Self::peer_transfer`] rather than building the
        /// variant by hand.
        counterparty: Option<String>,
    },
}

impl TransferPattern {
    /// A peer transfer, with a blank counterparty read as unknown.
    ///
    /// Canonicalising here means two descriptions that differ only in spacing or
    /// case teach, and match, the same rule.
    #[must_use]
    pub fn peer_transfer(counterparty: Option<&str>) -> Self {
        let counterparty = counterparty
            .map(|value| value.trim().to_ascii_uppercase())
            .filter(|value| !value.is_empty());
        Self::PeerTransfer { counterparty }
    }

    /// The counterparty, when the description named one.
    #[must_use]
    pub fn counterparty(&self) -> Option<&str> {
        match self {
            Self::CashWithdrawal => None,
            Self::PeerTransfer { counterparty } => counterparty.as_deref(),
        }
    }
}

/// What a remembered rule matches on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleKey {
    /// A merchant name, canonical: trimmed and ASCII upper-case.
    Merchant(String),

    /// A kind of own-account movement.
    Transfer(TransferPattern),
}

impl RuleKey {
    /// A merchant key, or `None` when the name is blank.
    ///
    /// A blank key would match every row that has no merchant, which is how one
    /// correction becomes forty wrong ones, so it is refused rather than stored.
    #[must_use]
    pub fn merchant(name: &str) -> Option<Self> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        Some(Self::Merchant(name.to_ascii_uppercase()))
    }

    /// A transfer key.
    #[must_use]
    pub const fn transfer(pattern: TransferPattern) -> Self {
        Self::Transfer(pattern)
    }
}

/// What a matching row becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuleTarget {
    /// Spending or income filed under a category.
    Category(CategoryId),

    /// Money moved to another of the owner's accounts.
    Account(AccountId),
}

/// A choice the app applies to rows that look the same.
///
/// Whether the key can actually be matched against a row is the importer's
/// knowledge, and whether the target exists is the ledger's: this type only
/// carries the pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    key: RuleKey,
    target: RuleTarget,
}

impl Rule {
    /// Pairs a key with what a matching row becomes.
    #[must_use]
    pub const fn new(key: RuleKey, target: RuleTarget) -> Self {
        Self { key, target }
    }

    /// What this rule matches on.
    #[must_use]
    pub const fn key(&self) -> &RuleKey {
        &self.key
    }

    /// What a matching row becomes.
    #[must_use]
    pub const fn target(&self) -> RuleTarget {
        self.target
    }
}
