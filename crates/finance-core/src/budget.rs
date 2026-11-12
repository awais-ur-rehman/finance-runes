//! Budgets: a monthly spending limit for one category.
//!
//! A budget is **dated**. It takes effect in one month and stays in force until
//! a later change replaces it, so a month that has already happened always keeps
//! the limit that was actually in force at the time (FR-054).
//!
//! A budget on a parent category covers its children: budgeting "Food" counts
//! what was spent on Groceries and Restaurants. That is what the word means to a
//! person, and the alternative — reporting zero while the money goes out — would
//! be silently wrong.

use crate::ledger::YearMonth;
use crate::{CategoryId, Money};

/// The default share of a budget at which a category counts as near its limit.
pub const DEFAULT_NEAR_LIMIT_PERCENT: u8 = 90;

/// A spending limit for one category, in force from a given month onward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    category: CategoryId,
    amount: Money,
    effective_from: YearMonth,
}

impl Budget {
    /// Creates a budget in force from `effective_from` until replaced.
    pub const fn new(category: CategoryId, amount: Money, effective_from: YearMonth) -> Self {
        Self {
            category,
            amount,
            effective_from,
        }
    }

    /// The category this limit applies to.
    pub const fn category(self) -> CategoryId {
        self.category
    }

    /// The monthly limit.
    pub const fn amount(self) -> Money {
        self.amount
    }

    /// The first month this limit applies to.
    pub const fn effective_from(self) -> YearMonth {
        self.effective_from
    }
}

/// How a category is doing against its limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetState {
    /// Spending is below the near-limit threshold.
    Within,
    /// Spending has reached the threshold but not the limit.
    NearLimit,
    /// Spending is above the limit.
    Over,
}

/// One category's position for a month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetLine {
    category: CategoryId,
    budgeted: Money,
    spent: Money,
    remaining: Money,
    state: BudgetState,
}

impl BudgetLine {
    pub(crate) const fn new(
        category: CategoryId,
        budgeted: Money,
        spent: Money,
        remaining: Money,
        state: BudgetState,
    ) -> Self {
        Self {
            category,
            budgeted,
            spent,
            remaining,
            state,
        }
    }

    /// The budgeted category.
    pub const fn category(self) -> CategoryId {
        self.category
    }

    /// The limit in force for the month.
    pub const fn budgeted(self) -> Money {
        self.budgeted
    }

    /// Spending in the category, including its child categories.
    pub const fn spent(self) -> Money {
        self.spent
    }

    /// What is left of the limit. Negative when the category is over budget.
    pub const fn remaining(self) -> Money {
        self.remaining
    }

    /// How the category is doing against its limit.
    pub const fn state(self) -> BudgetState {
        self.state
    }
}

/// Every budgeted category's position for one month, plus the totals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetReport {
    period: YearMonth,
    lines: Vec<BudgetLine>,
    budgeted: Money,
    spent: Money,
    remaining: Money,
    surplus: Money,
}

impl BudgetReport {
    pub(crate) const fn new(
        period: YearMonth,
        lines: Vec<BudgetLine>,
        budgeted: Money,
        spent: Money,
        remaining: Money,
        surplus: Money,
    ) -> Self {
        Self {
            period,
            lines,
            budgeted,
            spent,
            remaining,
            surplus,
        }
    }

    /// The month reported on.
    pub const fn period(&self) -> YearMonth {
        self.period
    }

    /// One line per budgeted category, ordered by category identifier.
    pub fn lines(&self) -> &[BudgetLine] {
        &self.lines
    }

    /// The total of every limit in force.
    pub const fn budgeted(&self) -> Money {
        self.budgeted
    }

    /// Total spending across budgeted categories.
    pub const fn spent(&self) -> Money {
        self.spent
    }

    /// Budgeted minus spent. Negative when the month is over budget overall.
    pub const fn remaining(&self) -> Money {
        self.remaining
    }

    /// What is left to offer as a transfer to savings: `remaining`, or zero.
    ///
    /// An overspent category reduces the offer, because it is spending that the
    /// budget did not cover. The offer never goes below zero (FR-053).
    pub const fn surplus(&self) -> Money {
        self.surplus
    }

    /// True when any category is over its limit.
    pub fn is_over_budget(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.state() == BudgetState::Over)
    }
}
