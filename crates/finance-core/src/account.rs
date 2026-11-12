//! Accounts: the real places money sits.

use crate::AccountId;
use crate::money::Currency;

/// What an account represents.
///
/// Only [`AccountKind::Bank`] and [`AccountKind::Cash`] need UI in v1; the other
/// kinds exist so the model does not have to change when they are added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccountKind {
    /// A bank account, usually with a debit card.
    Bank,
    /// Physical cash.
    Cash,
    /// A savings pot that is not spent from.
    Savings,
    /// A credit card: money owed rather than held.
    Card,
    /// A mobile wallet such as JazzCash or Easypaisa.
    Wallet,
}

impl std::fmt::Display for AccountKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Bank => "bank",
            Self::Cash => "cash",
            Self::Savings => "savings",
            Self::Card => "card",
            Self::Wallet => "wallet",
        };
        f.write_str(name)
    }
}

/// A place money sits, in exactly one currency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    name: String,
    kind: AccountKind,
    currency: Currency,
    archived: bool,
}

impl Account {
    /// Creates an active account.
    pub fn new(
        id: AccountId,
        name: impl Into<String>,
        kind: AccountKind,
        currency: Currency,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            currency,
            archived: false,
        }
    }

    /// The account's identifier.
    pub const fn id(&self) -> AccountId {
        self.id
    }

    /// The account's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// What kind of account this is.
    pub const fn kind(&self) -> AccountKind {
        self.kind
    }

    /// The account's currency.
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// True when the account is archived: hidden from pickers, kept in history.
    pub const fn is_archived(&self) -> bool {
        self.archived
    }

    /// Renames the account.
    pub fn rename(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Archives the account.
    pub fn archive(&mut self) {
        self.archived = true;
    }
}
