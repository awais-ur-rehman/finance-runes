//! Integration tests for the ledger.
//!
//! These cover the rules a transaction must satisfy before it is recorded, and
//! the summaries derived from the recorded ones. They deliberately go through the
//! public API only, the same way the storage layer and the UI will.

use finance_core::{
    Account, AccountId, AccountKind, Category, CategoryId, CategoryKind, Currency, Ledger,
    LedgerError, Money, Transaction, TransactionError, TransactionId, YearMonth,
};
use time::{Date, Month};
use uuid::Uuid;

const NOVEMBER: Month = Month::November;

fn day(number: u8) -> Date {
    Date::from_calendar_date(2026, NOVEMBER, number).expect("a valid November date")
}

fn month() -> YearMonth {
    YearMonth::new(2026, NOVEMBER)
}

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

fn account_id(value: u128) -> AccountId {
    AccountId::from_uuid(Uuid::from_u128(value))
}

fn category_id(value: u128) -> CategoryId {
    CategoryId::from_uuid(Uuid::from_u128(value))
}

fn transaction_id(value: u128) -> TransactionId {
    TransactionId::from_uuid(Uuid::from_u128(value))
}

struct Fixture {
    ledger: Ledger,
    bank: AccountId,
    cash: AccountId,
    food: CategoryId,
    groceries: CategoryId,
    salary: CategoryId,
}

fn fixture() -> Fixture {
    let bank = account_id(1);
    let cash = account_id(2);
    let food = category_id(1);
    let groceries = category_id(2);
    let salary = category_id(3);

    let mut ledger = Ledger::new(Currency::PKR);
    ledger
        .add_account(Account::new(bank, "Bank", AccountKind::Bank, Currency::PKR))
        .expect("the first account is unique");
    ledger
        .add_account(Account::new(cash, "Cash", AccountKind::Cash, Currency::PKR))
        .expect("the second account is unique");
    ledger
        .add_category(Category::new(food, "Food", CategoryKind::Expense))
        .expect("the parent category is unique");
    ledger
        .add_category(
            Category::new(groceries, "Groceries", CategoryKind::Expense).with_parent(food),
        )
        .expect("the child category is valid");
    ledger
        .add_category(Category::new(salary, "Salary", CategoryKind::Income))
        .expect("the income category is unique");

    Fixture {
        ledger,
        bank,
        cash,
        food,
        groceries,
        salary,
    }
}

#[test]
fn a_batch_of_transactions_is_recorded_atomically() {
    let mut fixture = fixture();
    let before = fixture.ledger.clone();

    let good = Transaction::expense(
        transaction_id(1),
        day(2),
        fixture.bank,
        fixture.food,
        pkr(10_000),
    )
    .expect("the amount is positive");
    let wrong_kind = Transaction::expense(
        transaction_id(2),
        day(3),
        fixture.bank,
        fixture.salary,
        pkr(10_000),
    )
    .expect("the amount is positive");

    let error = fixture
        .ledger
        .record_all([good, wrong_kind])
        .expect_err("an expense cannot use an income category");

    assert!(matches!(
        error,
        LedgerError::CategoryKindMismatch {
            expected: CategoryKind::Expense,
            found: CategoryKind::Income
        }
    ));
    assert_eq!(
        fixture.ledger, before,
        "the first transaction must not survive a failed batch"
    );
}

#[test]
fn a_batch_of_valid_transactions_is_recorded() {
    let mut fixture = fixture();

    fixture
        .ledger
        .record_all([
            Transaction::expense(
                transaction_id(1),
                day(2),
                fixture.bank,
                fixture.food,
                pkr(10_000),
            )
            .expect("the amount is positive"),
            Transaction::income(
                transaction_id(2),
                day(3),
                fixture.bank,
                fixture.salary,
                pkr(50_000),
            )
            .expect("the amount is positive"),
        ])
        .expect("both transactions are valid");

    assert_eq!(fixture.ledger.transactions().count(), 2);
}

#[test]
fn a_month_summarises_income_and_expenses() {
    let mut fixture = fixture();
    let income = pkr(15_000_000); // 150,000.00
    let rent = pkr(4_000_000); // 40,000.00
    let shopping = pkr(1_250_000); // 12,500.00

    fixture
        .ledger
        .record(
            Transaction::income(
                transaction_id(1),
                day(1),
                fixture.bank,
                fixture.salary,
                income,
            )
            .expect("income is positive"),
        )
        .expect("the income category matches");
    fixture
        .ledger
        .record(
            Transaction::expense(transaction_id(2), day(2), fixture.bank, fixture.food, rent)
                .expect("the amount is positive"),
        )
        .expect("the expense category matches");
    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(3),
                day(3),
                fixture.cash,
                fixture.groceries,
                shopping,
            )
            .expect("the amount is positive"),
        )
        .expect("the expense category matches");

    let summary = fixture.ledger.summary(month()).expect("totals fit");

    assert_eq!(summary.period(), month());
    assert_eq!(summary.income(), income);
    assert_eq!(
        summary.expenses(),
        rent.checked_add(shopping).expect("fits")
    );
    assert_eq!(summary.saved(), pkr(9_750_000)); // 97,500.00
    assert_eq!(summary.savings_rate_percent(), Some(65.0));
}

#[test]
fn transfers_leave_income_and_expenses_untouched() {
    let mut fixture = fixture();
    let income = pkr(15_000_000);
    let rent = pkr(4_000_000);

    fixture
        .ledger
        .record(
            Transaction::income(
                transaction_id(1),
                day(1),
                fixture.bank,
                fixture.salary,
                income,
            )
            .expect("income is positive"),
        )
        .expect("the income category matches");
    fixture
        .ledger
        .record(
            Transaction::expense(transaction_id(2), day(2), fixture.bank, fixture.food, rent)
                .expect("the amount is positive"),
        )
        .expect("the expense category matches");

    let before = fixture.ledger.summary(month()).expect("totals fit");

    fixture
        .ledger
        .record(
            Transaction::transfer(
                transaction_id(3),
                day(5),
                fixture.bank,
                fixture.cash,
                pkr(2_000_000),
            )
            .expect("the amount is positive"),
        )
        .expect("both accounts exist");

    let after = fixture.ledger.summary(month()).expect("totals fit");

    assert_eq!(before, after, "a transfer is not income or an expense");
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.bank, day(30))
            .expect("known account"),
        pkr(9_000_000),
        "the bank lost the transfer"
    );
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.cash, day(30))
            .expect("known account"),
        pkr(2_000_000),
        "cash received the transfer"
    );
}

#[test]
fn an_opening_balance_seeds_an_account_without_touching_the_month() {
    let mut fixture = fixture();

    fixture
        .ledger
        .record(
            Transaction::opening(transaction_id(1), day(1), fixture.cash, pkr(500_000))
                .expect("the amount is positive"),
        )
        .expect("the account exists");

    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.cash, day(1))
            .expect("known account"),
        pkr(500_000)
    );
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.cash, day(30))
            .expect("known account"),
        pkr(500_000)
    );

    let summary = fixture.ledger.summary(month()).expect("totals fit");
    assert_eq!(summary.income(), pkr(0));
    assert_eq!(summary.expenses(), pkr(0));
    assert_eq!(summary.saved(), pkr(0));
}

#[test]
fn a_balance_ignores_transactions_dated_later() {
    let mut fixture = fixture();

    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(1),
                day(10),
                fixture.bank,
                fixture.food,
                pkr(1_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the expense category matches");
    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(2),
                day(20),
                fixture.bank,
                fixture.food,
                pkr(2_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the expense category matches");

    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.bank, day(9))
            .expect("known account"),
        pkr(0)
    );
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.bank, day(10))
            .expect("known account"),
        pkr(-1_000)
    );
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.bank, day(30))
            .expect("known account"),
        pkr(-3_000)
    );
}

#[test]
fn an_expense_against_an_unknown_account_is_rejected() {
    let mut fixture = fixture();
    let ghost = account_id(99);

    let transaction =
        Transaction::expense(transaction_id(1), day(1), ghost, fixture.food, pkr(100))
            .expect("the amount is positive");

    assert_eq!(
        fixture.ledger.record(transaction),
        Err(LedgerError::UnknownAccount(ghost))
    );
}

#[test]
fn an_expense_against_an_unknown_category_is_rejected() {
    let mut fixture = fixture();
    let ghost = category_id(99);

    let transaction =
        Transaction::expense(transaction_id(1), day(1), fixture.bank, ghost, pkr(100))
            .expect("the amount is positive");

    assert_eq!(
        fixture.ledger.record(transaction),
        Err(LedgerError::UnknownCategory(ghost))
    );
}

#[test]
fn an_expense_cannot_use_an_income_category() {
    let mut fixture = fixture();

    let transaction = Transaction::expense(
        transaction_id(1),
        day(1),
        fixture.bank,
        fixture.salary,
        pkr(100),
    )
    .expect("the amount is positive");

    assert_eq!(
        fixture.ledger.record(transaction),
        Err(LedgerError::CategoryKindMismatch {
            expected: CategoryKind::Expense,
            found: CategoryKind::Income,
        })
    );
}

#[test]
fn income_cannot_use_an_expense_category() {
    let mut fixture = fixture();

    let transaction = Transaction::income(
        transaction_id(1),
        day(1),
        fixture.bank,
        fixture.food,
        pkr(100),
    )
    .expect("the amount is positive");

    assert_eq!(
        fixture.ledger.record(transaction),
        Err(LedgerError::CategoryKindMismatch {
            expected: CategoryKind::Income,
            found: CategoryKind::Expense,
        })
    );
}

#[test]
fn a_transaction_cannot_be_recorded_twice() {
    let mut fixture = fixture();
    let id = transaction_id(1);

    let first = Transaction::expense(id, day(1), fixture.bank, fixture.food, pkr(100))
        .expect("the amount is positive");
    let second = Transaction::expense(id, day(2), fixture.bank, fixture.food, pkr(200))
        .expect("the amount is positive");

    fixture
        .ledger
        .record(first)
        .expect("the first record succeeds");
    assert_eq!(
        fixture.ledger.record(second),
        Err(LedgerError::DuplicateTransaction(id))
    );
}

#[test]
fn a_transfer_between_two_identical_accounts_is_rejected() {
    let mut fixture = fixture();

    let transaction = Transaction::transfer(
        transaction_id(1),
        day(1),
        fixture.bank,
        fixture.bank,
        pkr(100),
    )
    .expect("the amount is positive");

    assert_eq!(
        fixture.ledger.record(transaction),
        Err(LedgerError::DuplicateAccountPosting)
    );
}

#[test]
fn categories_may_not_nest_three_levels_deep() {
    let mut fixture = fixture();
    let dairy = category_id(4);

    let too_deep =
        Category::new(dairy, "Dairy", CategoryKind::Expense).with_parent(fixture.groceries);

    assert_eq!(
        fixture.ledger.add_category(too_deep),
        Err(LedgerError::CategoryTooDeep)
    );
}

#[test]
fn a_child_category_must_match_its_parents_kind() {
    let mut fixture = fixture();
    let bonus = category_id(4);

    let mismatched = Category::new(bonus, "Bonus", CategoryKind::Income).with_parent(fixture.food);

    assert_eq!(
        fixture.ledger.add_category(mismatched),
        Err(LedgerError::CategoryKindMismatch {
            expected: CategoryKind::Expense,
            found: CategoryKind::Income,
        })
    );
}

#[test]
fn a_category_needs_an_existing_parent() {
    let mut fixture = fixture();
    let child = category_id(4);
    let missing_parent = category_id(99);

    let orphan = Category::new(child, "Orphan", CategoryKind::Expense).with_parent(missing_parent);

    assert_eq!(
        fixture.ledger.add_category(orphan),
        Err(LedgerError::UnknownParent(missing_parent))
    );
}

#[test]
fn an_account_must_share_the_ledgers_currency() {
    let mut fixture = fixture();
    let foreign = account_id(50);

    assert_eq!(
        fixture.ledger.add_account(Account::new(
            foreign,
            "Dollar",
            AccountKind::Bank,
            Currency::USD
        )),
        Err(LedgerError::AccountCurrencyMismatch {
            account: Currency::USD,
            ledger: Currency::PKR,
        })
    );
}

#[test]
fn an_expense_must_carry_a_positive_amount() {
    let fixture = fixture();

    for amount in [pkr(0), pkr(-100)] {
        assert_eq!(
            Transaction::expense(
                transaction_id(1),
                day(1),
                fixture.bank,
                fixture.food,
                amount
            ),
            Err(TransactionError::NonPositiveAmount)
        );
    }
}

#[test]
fn the_savings_rate_is_absent_when_nothing_was_earned() {
    let mut fixture = fixture();

    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(1),
                day(1),
                fixture.bank,
                fixture.food,
                pkr(1_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the expense category matches");

    let summary = fixture.ledger.summary(month()).expect("totals fit");

    assert_eq!(summary.saved(), pkr(-1_000));
    assert_eq!(
        summary.savings_rate_percent(),
        None,
        "a rate against zero income is undefined, not infinite"
    );
}

#[test]
fn spending_is_reported_per_category() {
    let mut fixture = fixture();

    for (id, category, amount) in [
        (1, fixture.groceries, pkr(1_500)),
        (2, fixture.groceries, pkr(1_000)),
        (3, fixture.food, pkr(400)),
    ] {
        fixture
            .ledger
            .record(
                Transaction::expense(transaction_id(id), day(1), fixture.bank, category, amount)
                    .expect("the amount is positive"),
            )
            .expect("the category matches");
    }

    let spending = fixture.ledger.spending_in(month()).expect("totals fit");

    assert_eq!(spending.get(&fixture.groceries), Some(&pkr(2_500)));
    assert_eq!(spending.get(&fixture.food), Some(&pkr(400)));
    assert_eq!(spending.get(&fixture.salary), None);
}

#[test]
fn spending_ignores_other_months_and_other_transaction_kinds() {
    let mut fixture = fixture();

    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(1),
                day(1),
                fixture.bank,
                fixture.food,
                pkr(1_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the category matches");
    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(2),
                Date::from_calendar_date(2026, Month::October, 31).expect("a valid date"),
                fixture.bank,
                fixture.food,
                pkr(9_999),
            )
            .expect("the amount is positive"),
        )
        .expect("the category matches");
    fixture
        .ledger
        .record(
            Transaction::income(
                transaction_id(3),
                day(2),
                fixture.bank,
                fixture.salary,
                pkr(5_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the category matches");

    let spending = fixture.ledger.spending_in(month()).expect("totals fit");

    assert_eq!(spending.len(), 1);
    assert_eq!(spending.get(&fixture.food), Some(&pkr(1_000)));
}

#[test]
fn the_account_balances_and_category_totals_cancel_out() {
    let mut fixture = fixture();

    fixture
        .ledger
        .record(
            Transaction::income(
                transaction_id(1),
                day(1),
                fixture.bank,
                fixture.salary,
                pkr(10_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the category matches");
    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(2),
                day(2),
                fixture.bank,
                fixture.food,
                pkr(3_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the category matches");
    fixture
        .ledger
        .record(
            Transaction::transfer(
                transaction_id(3),
                day(3),
                fixture.bank,
                fixture.cash,
                pkr(2_000),
            )
            .expect("the amount is positive"),
        )
        .expect("both accounts exist");

    let accounts = fixture
        .ledger
        .total_balance_as_of(day(30))
        .expect("totals fit");
    let categories = fixture
        .ledger
        .category_totals()
        .expect("totals fit")
        .values()
        .fold(pkr(0), |total, amount| {
            total.checked_add(*amount).expect("totals fit")
        });

    assert_eq!(
        accounts,
        pkr(7_000),
        "bank 5,000 plus cash 2,000, net of the transfer"
    );
    assert_eq!(
        categories,
        pkr(-7_000),
        "expense categories are positive, income categories negative"
    );
}
