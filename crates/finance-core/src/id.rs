//! Stable identifiers. Identifiers are supplied by the caller so the domain
//! never has to reach for randomness.

use uuid::Uuid;

/// Identifier for an [`Account`](crate::Account).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(Uuid);

impl AccountId {
    /// Wraps an existing UUID.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// The underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Identifier for a [`Category`](crate::Category).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CategoryId(Uuid);

impl CategoryId {
    /// Wraps an existing UUID.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// The underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for CategoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Identifier for a [`Transaction`](crate::Transaction).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransactionId(Uuid);

impl TransactionId {
    /// Wraps an existing UUID.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// The underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
