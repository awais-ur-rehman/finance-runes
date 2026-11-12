//! Transactions and their postings.
//!
//! A transaction is a set of postings that sum to zero. That is what makes a
//! transfer provably neutral and a split provably exact (ADR-0005). The UI never
//! shows postings; it shows "−450.00 · Groceries".

use crate::error::TransactionError;
use crate::{AccountId, CategoryId, Money, TransactionId};
use time::Date;

/// What a posting applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PostingTarget {
    /// A real account, such as the bank account or cash.
    Account(AccountId),
    /// A category, which acts as a virtual expense or income account.
    Category(CategoryId),
    /// The virtual account that records an account's opening balance.
    Opening,
}

/// One side of a transaction: an amount applied to a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Posting {
    target: PostingTarget,
    amount: Money,
}

impl Posting {
    /// Builds a posting. Sign is meaningful: positive increases the target,
    /// negative decreases it.
    pub const fn new(target: PostingTarget, amount: Money) -> Self {
        Self { target, amount }
    }

    /// What this posting applies to.
    pub const fn target(self) -> PostingTarget {
        self.target
    }

    /// The signed amount.
    pub const fn amount(self) -> Money {
        self.amount
    }
}

/// The business meaning of a transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransactionKind {
    /// Money leaving: one account posting and one expense category posting.
    Expense,
    /// Money arriving: one account posting and one income category posting.
    Income,
    /// Money moving between two of your own accounts.
    Transfer,
    /// An account's balance before the first recorded transaction.
    Opening,
}

impl std::fmt::Display for TransactionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Expense => "expense",
            Self::Income => "income",
            Self::Transfer => "transfer",
            Self::Opening => "opening",
        };
        f.write_str(name)
    }
}

/// A dated event that moves money.
///
/// Construction guarantees the postings are balanced and share one currency.
/// Whether the referenced accounts and categories exist, and whether their kinds
/// match, is the ledger's job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    id: TransactionId,
    date: Date,
    kind: TransactionKind,
    payee: Option<String>,
    note: Option<String>,
    postings: Vec<Posting>,
    amount: Money,
}

impl Transaction {
    /// Builds a transaction from arbitrary balanced postings.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::TooFewPostings`] for fewer than two postings,
    /// [`TransactionError::CurrencyMismatch`] when the postings disagree on
    /// currency, [`TransactionError::Unbalanced`] when they do not net to zero,
    /// and [`TransactionError::Overflow`] when the total does not fit.
    pub fn new(
        id: TransactionId,
        date: Date,
        kind: TransactionKind,
        postings: Vec<Posting>,
    ) -> Result<Self, TransactionError> {
        if postings.len() < 2 {
            return Err(TransactionError::TooFewPostings);
        }

        let currency = postings[0].amount().currency();
        let mut net: i128 = 0;
        let mut positive: i128 = 0;

        for posting in &postings {
            let amount = posting.amount();
            if amount.currency() != currency {
                return Err(TransactionError::CurrencyMismatch);
            }
            net += i128::from(amount.minor());
            if amount.is_positive() {
                positive += i128::from(amount.minor());
            }
        }

        if net != 0 {
            return Err(TransactionError::Unbalanced { minor: net });
        }

        let total = i64::try_from(positive).map_err(|_| TransactionError::Overflow)?;

        Ok(Self {
            id,
            date,
            kind,
            payee: None,
            note: None,
            postings,
            amount: Money::from_minor(total, currency),
        })
    }

    /// Records money leaving an account for an expense category.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::NonPositiveAmount`] when `amount` is zero or
    /// negative.
    pub fn expense(
        id: TransactionId,
        date: Date,
        account: AccountId,
        category: CategoryId,
        amount: Money,
    ) -> Result<Self, TransactionError> {
        if !amount.is_positive() {
            return Err(TransactionError::NonPositiveAmount);
        }
        let outflow = amount
            .checked_neg()
            .map_err(|_| TransactionError::Overflow)?;
        let postings = vec![
            Posting::new(PostingTarget::Account(account), outflow),
            Posting::new(PostingTarget::Category(category), amount),
        ];
        Self::new(id, date, TransactionKind::Expense, postings)
    }

    /// Records money arriving in an account from an income category.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::NonPositiveAmount`] when `amount` is zero or
    /// negative.
    pub fn income(
        id: TransactionId,
        date: Date,
        account: AccountId,
        category: CategoryId,
        amount: Money,
    ) -> Result<Self, TransactionError> {
        if !amount.is_positive() {
            return Err(TransactionError::NonPositiveAmount);
        }
        let outflow = amount
            .checked_neg()
            .map_err(|_| TransactionError::Overflow)?;
        let postings = vec![
            Posting::new(PostingTarget::Account(account), amount),
            Posting::new(PostingTarget::Category(category), outflow),
        ];
        Self::new(id, date, TransactionKind::Income, postings)
    }

    /// Moves money between two of your own accounts.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::NonPositiveAmount`] when `amount` is zero or
    /// negative.
    pub fn transfer(
        id: TransactionId,
        date: Date,
        from: AccountId,
        to: AccountId,
        amount: Money,
    ) -> Result<Self, TransactionError> {
        if !amount.is_positive() {
            return Err(TransactionError::NonPositiveAmount);
        }
        let outflow = amount
            .checked_neg()
            .map_err(|_| TransactionError::Overflow)?;
        let postings = vec![
            Posting::new(PostingTarget::Account(from), outflow),
            Posting::new(PostingTarget::Account(to), amount),
        ];
        Self::new(id, date, TransactionKind::Transfer, postings)
    }

    /// Records an account's opening balance.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::NonPositiveAmount`] when `amount` is zero or
    /// negative.
    pub fn opening(
        id: TransactionId,
        date: Date,
        account: AccountId,
        amount: Money,
    ) -> Result<Self, TransactionError> {
        if !amount.is_positive() {
            return Err(TransactionError::NonPositiveAmount);
        }
        let outflow = amount
            .checked_neg()
            .map_err(|_| TransactionError::Overflow)?;
        let postings = vec![
            Posting::new(PostingTarget::Account(account), amount),
            Posting::new(PostingTarget::Opening, outflow),
        ];
        Self::new(id, date, TransactionKind::Opening, postings)
    }

    /// Attaches the merchant or income source.
    #[must_use]
    pub fn with_payee(mut self, payee: impl Into<String>) -> Self {
        self.payee = Some(payee.into());
        self
    }

    /// Attaches a free-text note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The transaction's identifier.
    pub const fn id(&self) -> TransactionId {
        self.id
    }

    /// The date the transaction applies to.
    pub const fn date(&self) -> Date {
        self.date
    }

    /// The business meaning of the transaction.
    pub const fn kind(&self) -> TransactionKind {
        self.kind
    }

    /// The merchant or income source, if recorded.
    pub fn payee(&self) -> Option<&str> {
        self.payee.as_deref()
    }

    /// The free-text note, if recorded.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// The postings, in the order they were built.
    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }

    /// The absolute amount moved by the transaction.
    pub const fn amount(&self) -> Money {
        self.amount
    }
}
