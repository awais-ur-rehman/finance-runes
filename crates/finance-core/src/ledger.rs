//! The ledger: an append-only record of transactions, plus derived views.
//!
//! Balances are always derived from postings. Nothing here stores a balance as
//! truth, so a bug in a report cannot destroy history.

use std::collections::BTreeMap;

use time::{Date, Month};

use crate::budget::{Budget, BudgetLine, BudgetReport, BudgetState};
use crate::error::{LedgerError, MoneyError};
use crate::{
    Account, AccountId, Category, CategoryId, CategoryKind, Currency, Money, PostingTarget,
    Rounding, Transaction, TransactionId, TransactionKind,
};

/// A calendar month, used for summaries, budgets and insights.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    year: i32,
    month: Month,
}

impl YearMonth {
    /// Builds a month from a year and a month.
    pub const fn new(year: i32, month: Month) -> Self {
        Self { year, month }
    }

    /// The month a date falls in.
    #[must_use]
    pub fn of(date: Date) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    /// The year.
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month.
    pub const fn month(self) -> Month {
        self.month
    }

    /// True when `date` falls in this month.
    #[must_use]
    pub fn contains(self, date: Date) -> bool {
        date.year() == self.year && date.month() == self.month
    }
}

impl std::fmt::Display for YearMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}", self.year, u8::from(self.month))
    }
}

/// Income, expenses and what was saved in one month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthSummary {
    period: YearMonth,
    income: Money,
    expenses: Money,
    saved: Money,
}

impl MonthSummary {
    /// The month summarised.
    pub const fn period(self) -> YearMonth {
        self.period
    }

    /// Total income in the month.
    pub const fn income(self) -> Money {
        self.income
    }

    /// Total expenses in the month.
    pub const fn expenses(self) -> Money {
        self.expenses
    }

    /// Income minus expenses. Transfers never affect this.
    pub const fn saved(self) -> Money {
        self.saved
    }

    /// Saved as a percentage of income, or `None` when there was no income.
    ///
    /// The rate is display-only arithmetic, which is why it may be a float.
    #[must_use]
    pub fn savings_rate_percent(self) -> Option<f64> {
        if self.income.minor() <= 0 {
            return None;
        }
        Some(self.saved.minor() as f64 / self.income.minor() as f64 * 100.0)
    }
}

/// The record of everything that happened, and the views derived from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ledger {
    currency: Currency,
    accounts: BTreeMap<AccountId, Account>,
    categories: BTreeMap<CategoryId, Category>,
    transactions: Vec<Transaction>,
    budgets: BTreeMap<(CategoryId, YearMonth), Budget>,
}

impl Ledger {
    /// Creates an empty ledger in one currency.
    pub fn new(currency: Currency) -> Self {
        Self {
            currency,
            accounts: BTreeMap::new(),
            categories: BTreeMap::new(),
            transactions: Vec::new(),
            budgets: BTreeMap::new(),
        }
    }

    /// The ledger's currency.
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// All accounts, including archived ones, ordered by identifier.
    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    /// All categories, ordered by identifier.
    pub fn categories(&self) -> impl Iterator<Item = &Category> {
        self.categories.values()
    }

    /// All transactions, oldest first in insertion order.
    pub fn transactions(&self) -> impl Iterator<Item = &Transaction> {
        self.transactions.iter()
    }

    /// Looks up an account.
    pub fn account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.get(&id)
    }

    /// Looks up a category.
    pub fn category(&self, id: CategoryId) -> Option<&Category> {
        self.categories.get(&id)
    }

    /// Adds an account.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::DuplicateAccount`] if the identifier is taken and
    /// [`LedgerError::AccountCurrencyMismatch`] if the currencies differ.
    pub fn add_account(&mut self, account: Account) -> Result<(), LedgerError> {
        let id = account.id();
        if self.accounts.contains_key(&id) {
            return Err(LedgerError::DuplicateAccount(id));
        }
        if account.currency() != self.currency {
            return Err(LedgerError::AccountCurrencyMismatch {
                account: account.currency(),
                ledger: self.currency,
            });
        }
        self.accounts.insert(id, account);
        Ok(())
    }

    /// Adds a category, validating its parent and depth.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::DuplicateCategory`] if the identifier is taken,
    /// [`LedgerError::UnknownParent`] if the parent is missing,
    /// [`LedgerError::CategoryTooDeep`] for a third level, and
    /// [`LedgerError::CategoryKindMismatch`] when parent and child disagree.
    pub fn add_category(&mut self, category: Category) -> Result<(), LedgerError> {
        let id = category.id();
        if self.categories.contains_key(&id) {
            return Err(LedgerError::DuplicateCategory(id));
        }
        if let Some(parent_id) = category.parent() {
            let parent = self
                .categories
                .get(&parent_id)
                .ok_or(LedgerError::UnknownParent(parent_id))?;
            if parent.parent().is_some() {
                return Err(LedgerError::CategoryTooDeep);
            }
            if parent.kind() != category.kind() {
                return Err(LedgerError::CategoryKindMismatch {
                    expected: parent.kind(),
                    found: category.kind(),
                });
            }
        }
        self.categories.insert(id, category);
        Ok(())
    }

    /// Appends a transaction after validating the accounts and categories it
    /// references.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::DuplicateTransaction`] for a repeated identifier,
    /// [`LedgerError::UnknownAccount`] or [`LedgerError::UnknownCategory`] for a
    /// missing reference, [`LedgerError::CategoryKindMismatch`] when an expense
    /// targets an income category (or the reverse),
    /// [`LedgerError::DuplicateAccountPosting`] when an account appears twice,
    /// and [`LedgerError::InvalidShape`] when the postings do not fit the kind.
    pub fn record(&mut self, transaction: Transaction) -> Result<(), LedgerError> {
        if self
            .transactions
            .iter()
            .any(|existing| existing.id() == transaction.id())
        {
            return Err(LedgerError::DuplicateTransaction(transaction.id()));
        }

        let kind = transaction.kind();
        let mut accounts_seen: Vec<AccountId> = Vec::new();
        let mut category_postings = 0_usize;
        let mut opening_postings = 0_usize;

        for posting in transaction.postings() {
            match posting.target() {
                PostingTarget::Account(id) => {
                    let account = self
                        .accounts
                        .get(&id)
                        .ok_or(LedgerError::UnknownAccount(id))?;
                    if account.currency() != self.currency {
                        return Err(LedgerError::AccountCurrencyMismatch {
                            account: account.currency(),
                            ledger: self.currency,
                        });
                    }
                    if accounts_seen.contains(&id) {
                        return Err(LedgerError::DuplicateAccountPosting);
                    }
                    accounts_seen.push(id);
                }
                PostingTarget::Category(id) => {
                    let category = self
                        .categories
                        .get(&id)
                        .ok_or(LedgerError::UnknownCategory(id))?;
                    let expected = match kind {
                        TransactionKind::Expense => CategoryKind::Expense,
                        TransactionKind::Income => CategoryKind::Income,
                        TransactionKind::Transfer | TransactionKind::Opening => {
                            return Err(LedgerError::InvalidShape { kind });
                        }
                    };
                    if category.kind() != expected {
                        return Err(LedgerError::CategoryKindMismatch {
                            expected,
                            found: category.kind(),
                        });
                    }
                    category_postings += 1;
                }
                PostingTarget::Opening => {
                    if kind != TransactionKind::Opening {
                        return Err(LedgerError::InvalidShape { kind });
                    }
                    opening_postings += 1;
                }
            }
        }

        match kind {
            TransactionKind::Expense | TransactionKind::Income => {
                if accounts_seen.len() != 1 || category_postings != 1 || opening_postings != 0 {
                    return Err(LedgerError::InvalidShape { kind });
                }
            }
            TransactionKind::Transfer => {
                if accounts_seen.len() != 2 || category_postings != 0 || opening_postings != 0 {
                    return Err(LedgerError::InvalidShape { kind });
                }
            }
            TransactionKind::Opening => {
                if accounts_seen.len() != 1 || category_postings != 0 || opening_postings != 1 {
                    return Err(LedgerError::InvalidShape { kind });
                }
            }
        }

        self.transactions.push(transaction);
        Ok(())
    }

    /// An account's balance as of the end of `date`.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::UnknownAccount`] for an unknown account and
    /// [`LedgerError::Money`] if a total overflows.
    pub fn balance_as_of(&self, account: AccountId, date: Date) -> Result<Money, LedgerError> {
        if !self.accounts.contains_key(&account) {
            return Err(LedgerError::UnknownAccount(account));
        }
        let mut total = Money::zero(self.currency);
        for transaction in self.transactions.iter().filter(|item| item.date() <= date) {
            for posting in transaction.postings() {
                if posting.target() == PostingTarget::Account(account) {
                    total = total.checked_add(posting.amount())?;
                }
            }
        }
        Ok(total)
    }

    /// The sum of every account's balance as of `date`.
    ///
    /// # Errors
    ///
    /// Propagates [`LedgerError`] from [`Self::balance_as_of`].
    pub fn total_balance_as_of(&self, date: Date) -> Result<Money, LedgerError> {
        let mut total = Money::zero(self.currency);
        for account in self.accounts.values() {
            total = total.checked_add(self.balance_as_of(account.id(), date)?)?;
        }
        Ok(total)
    }

    /// The net total of every category posting, all time.
    ///
    /// Expense categories are positive, income categories negative. Together
    /// with the account balances this sums to zero, which is what makes the
    /// ledger auditable.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::Money`] if a total overflows.
    pub fn category_totals(&self) -> Result<BTreeMap<CategoryId, Money>, LedgerError> {
        let mut totals: BTreeMap<CategoryId, Money> = BTreeMap::new();
        for transaction in &self.transactions {
            for posting in transaction.postings() {
                if let PostingTarget::Category(id) = posting.target() {
                    let entry = totals
                        .entry(id)
                        .or_insert_with(|| Money::zero(self.currency));
                    *entry = entry.checked_add(posting.amount())?;
                }
            }
        }
        Ok(totals)
    }

    /// Spending per expense category in `period`.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::Money`] if a total overflows.
    pub fn spending_in(
        &self,
        period: YearMonth,
    ) -> Result<BTreeMap<CategoryId, Money>, LedgerError> {
        let mut totals: BTreeMap<CategoryId, Money> = BTreeMap::new();
        for transaction in self
            .transactions
            .iter()
            .filter(|item| item.kind() == TransactionKind::Expense && period.contains(item.date()))
        {
            for posting in transaction.postings() {
                if let PostingTarget::Category(id) = posting.target() {
                    let entry = totals
                        .entry(id)
                        .or_insert_with(|| Money::zero(self.currency));
                    *entry = entry.checked_add(posting.amount())?;
                }
            }
        }
        Ok(totals)
    }

    /// Income, expenses and savings for `period`.
    ///
    /// Transfers and opening balances are excluded by construction.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::Money`] if a total overflows.
    pub fn summary(&self, period: YearMonth) -> Result<MonthSummary, LedgerError> {
        let mut income = Money::zero(self.currency);
        let mut expenses = Money::zero(self.currency);

        for transaction in self
            .transactions
            .iter()
            .filter(|item| period.contains(item.date()))
        {
            match transaction.kind() {
                TransactionKind::Income => income = income.checked_add(transaction.amount())?,
                TransactionKind::Expense => {
                    expenses = expenses.checked_add(transaction.amount())?
                }
                TransactionKind::Transfer | TransactionKind::Opening => {}
            }
        }

        let saved = income.checked_sub(expenses)?;
        Ok(MonthSummary {
            period,
            income,
            expenses,
            saved,
        })
    }

    /// Looks up a transaction by identifier.
    pub fn transaction(&self, id: TransactionId) -> Option<&Transaction> {
        self.transactions.iter().find(|item| item.id() == id)
    }

    /// Every budget revision, ordered by category and then effective month.
    pub fn budgets(&self) -> impl Iterator<Item = &Budget> {
        self.budgets.values()
    }

    /// The budget in force for `category` in `period`, if any.
    ///
    /// The most recent revision that started on or before `period` wins, which is
    /// what keeps a past month on the limit that applied at the time (FR-054).
    pub fn budget_in_force(&self, category: CategoryId, period: YearMonth) -> Option<&Budget> {
        self.budgets
            .range(..=(category, period))
            .next_back()
            .filter(|((budgeted, _), _)| *budgeted == category)
            .map(|(_, budget)| budget)
    }

    /// Sets a monthly limit for `category`, in force from `effective_from`.
    ///
    /// Setting a second limit for the same category starts a new revision rather
    /// than editing history, so earlier months keep their old limit.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::UnknownCategory`] for an unknown category,
    /// [`LedgerError::CategoryKindMismatch`] when the category is not an expense
    /// category, [`LedgerError::Money`] when the amount is in another currency,
    /// [`LedgerError::NonPositiveBudget`] when the amount is zero or negative,
    /// and [`LedgerError::BudgetConflict`] when a parent or child category
    /// already has a budget.
    pub fn set_budget(
        &mut self,
        category: CategoryId,
        amount: Money,
        effective_from: YearMonth,
    ) -> Result<(), LedgerError> {
        let kind = self
            .categories
            .get(&category)
            .ok_or(LedgerError::UnknownCategory(category))?
            .kind();

        if kind != CategoryKind::Expense {
            return Err(LedgerError::CategoryKindMismatch {
                expected: CategoryKind::Expense,
                found: kind,
            });
        }

        if amount.currency() != self.currency {
            return Err(LedgerError::Money(MoneyError::CurrencyMismatch {
                left: amount.currency(),
                right: self.currency,
            }));
        }

        if !amount.is_positive() {
            return Err(LedgerError::NonPositiveBudget(category));
        }

        if let Some(other) = self.conflicting_budget(category) {
            return Err(LedgerError::BudgetConflict { category, other });
        }

        self.budgets.insert(
            (category, effective_from),
            Budget::new(category, amount, effective_from),
        );
        Ok(())
    }

    /// Removes every budget revision for `category`, returning how many went.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::UnknownCategory`] for an unknown category.
    pub fn clear_budget(&mut self, category: CategoryId) -> Result<usize, LedgerError> {
        if !self.categories.contains_key(&category) {
            return Err(LedgerError::UnknownCategory(category));
        }
        let before = self.budgets.len();
        self.budgets
            .retain(|(budgeted, _), _| *budgeted != category);
        Ok(before - self.budgets.len())
    }

    /// Spending in `category` and its child categories during `period`.
    ///
    /// A budget on a parent covers its children, so this is the figure a budget
    /// is measured against.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::UnknownCategory`] for an unknown category and
    /// [`LedgerError::Money`] if a total overflows.
    pub fn spent_on(&self, category: CategoryId, period: YearMonth) -> Result<Money, LedgerError> {
        if !self.categories.contains_key(&category) {
            return Err(LedgerError::UnknownCategory(category));
        }

        let direct = self.spending_in(period)?;
        let mut total = direct
            .get(&category)
            .copied()
            .unwrap_or_else(|| Money::zero(self.currency));

        for (id, child) in &self.categories {
            if child.parent() == Some(category)
                && let Some(amount) = direct.get(id)
            {
                total = total.checked_add(*amount)?;
            }
        }

        Ok(total)
    }

    /// The budget position for every budgeted category in `period`.
    ///
    /// `near_limit_percent` is the share of the limit at which a category is
    /// reported as near it, usually [`DEFAULT_NEAR_LIMIT_PERCENT`].
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::InvalidThreshold`] when the percentage is above
    /// 100 and [`LedgerError::Money`] if a total overflows.
    ///
    /// [`DEFAULT_NEAR_LIMIT_PERCENT`]: crate::budget::DEFAULT_NEAR_LIMIT_PERCENT
    pub fn budget_report(
        &self,
        period: YearMonth,
        near_limit_percent: u8,
    ) -> Result<BudgetReport, LedgerError> {
        if near_limit_percent > 100 {
            return Err(LedgerError::InvalidThreshold {
                percent: near_limit_percent,
            });
        }

        let mut lines = Vec::new();
        let mut budgeted = Money::zero(self.currency);
        let mut spent = Money::zero(self.currency);

        for category in self.budgeted_categories() {
            let Some(budget) = self.budget_in_force(category, period) else {
                continue;
            };
            let category_spent = self.spent_on(category, period)?;
            let remaining = budget.amount().checked_sub(category_spent)?;
            let state = classify(budget.amount(), category_spent, near_limit_percent);

            budgeted = budgeted.checked_add(budget.amount())?;
            spent = spent.checked_add(category_spent)?;
            lines.push(BudgetLine::new(
                category,
                budget.amount(),
                category_spent,
                remaining,
                state,
            ));
        }

        let remaining = budgeted.checked_sub(spent)?;
        let surplus = if remaining.is_positive() {
            remaining
        } else {
            Money::zero(self.currency)
        };

        Ok(BudgetReport::new(
            period, lines, budgeted, spent, remaining, surplus,
        ))
    }

    /// Copies the budgets in force in `from` into `to`, adjusted by a percentage.
    ///
    /// A `adjustment_percent` of `10` raises every limit by ten percent; `-5`
    /// lowers them. Returns how many budgets were copied (FR-055).
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::BudgetPeriodOrder`] when `to` is not after `from`,
    /// and propagates [`Self::set_budget`] for each copied limit.
    pub fn carry_budget_forward(
        &mut self,
        from: YearMonth,
        to: YearMonth,
        adjustment_percent: i16,
    ) -> Result<usize, LedgerError> {
        if to <= from {
            return Err(LedgerError::BudgetPeriodOrder { from, to });
        }

        let factor = i64::from(100_i16.saturating_add(adjustment_percent));
        let mut copies = Vec::new();

        for category in self.budgeted_categories() {
            if let Some(budget) = self.budget_in_force(category, from) {
                let adjusted = budget
                    .amount()
                    .checked_mul(factor)?
                    .div(100, Rounding::HalfUp)?;
                copies.push((category, adjusted));
            }
        }

        let count = copies.len();
        for (category, amount) in copies {
            self.set_budget(category, amount, to)?;
        }
        Ok(count)
    }

    /// Every category that has at least one budget revision, ordered.
    fn budgeted_categories(&self) -> Vec<CategoryId> {
        let mut ids: Vec<CategoryId> = self.budgets.keys().map(|(category, _)| *category).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// A parent or child category that already has a budget, if any.
    ///
    /// Budgets never end, so two budgets on the same branch would always overlap
    /// and double-count spending in the totals.
    fn conflicting_budget(&self, category: CategoryId) -> Option<CategoryId> {
        let mut ancestor = self.categories.get(&category).and_then(Category::parent);
        while let Some(id) = ancestor {
            if self.budgets.keys().any(|(budgeted, _)| *budgeted == id) {
                return Some(id);
            }
            ancestor = self.categories.get(&id).and_then(Category::parent);
        }

        for (id, other) in &self.categories {
            if other.parent() == Some(category)
                && self.budgets.keys().any(|(budgeted, _)| budgeted == id)
            {
                return Some(*id);
            }
        }

        None
    }
}

/// Classifies a category against its limit.
fn classify(budgeted: Money, spent: Money, near_limit_percent: u8) -> BudgetState {
    if spent.minor() > budgeted.minor() {
        return BudgetState::Over;
    }

    let spent = i128::from(spent.minor());
    let budgeted = i128::from(budgeted.minor());
    if spent * 100 >= budgeted * i128::from(near_limit_percent) {
        BudgetState::NearLimit
    } else {
        BudgetState::Within
    }
}
