//! Integration tests for budgets.
//!
//! A budget is dated and covers a category's children, so these tests are mostly
//! about time and hierarchy rather than arithmetic.

use finance_core::{
    Account, AccountId, AccountKind, BudgetReport, BudgetState, Category, CategoryId, CategoryKind,
    Currency, DEFAULT_NEAR_LIMIT_PERCENT, Ledger, LedgerError, Money, Transaction, TransactionId,
    YearMonth,
};
use time::{Date, Month};
use uuid::Uuid;

fn month(year: i32, month: Month) -> YearMonth {
    YearMonth::new(year, month)
}

fn day(month: Month, number: u8) -> Date {
    Date::from_calendar_date(2026, month, number).expect("a valid date")
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

struct Fixture {
    ledger: Ledger,
    bank: AccountId,
    cash: AccountId,
    food: CategoryId,
    groceries: CategoryId,
    restaurants: CategoryId,
    transport: CategoryId,
    salary: CategoryId,
}

fn fixture() -> Fixture {
    let bank = account_id(1);
    let cash = account_id(2);
    let food = category_id(1);
    let groceries = category_id(2);
    let restaurants = category_id(3);
    let transport = category_id(4);
    let salary = category_id(6);

    let mut ledger = Ledger::new(Currency::PKR);
    ledger
        .add_account(Account::new(bank, "Bank", AccountKind::Bank, Currency::PKR))
        .expect("the account is unique");
    ledger
        .add_account(Account::new(cash, "Cash", AccountKind::Cash, Currency::PKR))
        .expect("the account is unique");

    for (id, name, kind, parent) in [
        (food, "Food", CategoryKind::Expense, None),
        (groceries, "Groceries", CategoryKind::Expense, Some(food)),
        (
            restaurants,
            "Restaurants",
            CategoryKind::Expense,
            Some(food),
        ),
        (transport, "Transport", CategoryKind::Expense, None),
        (salary, "Salary", CategoryKind::Income, None),
    ] {
        let mut category = Category::new(id, name, kind);
        if let Some(parent) = parent {
            category = category.with_parent(parent);
        }
        ledger
            .add_category(category)
            .expect("the category is valid");
    }

    Fixture {
        ledger,
        bank,
        cash,
        food,
        groceries,
        restaurants,
        transport,
        salary,
    }
}

/// Records an expense, borrowing only the ledger so the caller can still read
/// the fixture's identifiers.
fn spend(
    ledger: &mut Ledger,
    account: AccountId,
    id: u128,
    category: CategoryId,
    amount: i64,
    on: Date,
) {
    let transaction = Transaction::expense(
        TransactionId::from_uuid(Uuid::from_u128(id)),
        on,
        account,
        category,
        pkr(amount),
    )
    .expect("the amount is positive");

    ledger.record(transaction).expect("the category matches");
}

fn report(fixture: &Fixture, period: YearMonth) -> BudgetReport {
    fixture
        .ledger
        .budget_report(period, DEFAULT_NEAR_LIMIT_PERCENT)
        .expect("the threshold is valid")
}

#[test]
fn a_budget_applies_to_every_month_until_changed() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.groceries,
            pkr(2_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");

    let august = report(&fixture, month(2026, Month::August));
    let november = report(&fixture, month(2026, Month::November));

    assert_eq!(august.budgeted(), pkr(2_000_000));
    assert_eq!(november.budgeted(), pkr(2_000_000));
}

#[test]
fn a_later_change_does_not_rewrite_an_earlier_month() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.groceries,
            pkr(2_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");
    fixture
        .ledger
        .set_budget(
            fixture.groceries,
            pkr(2_500_000),
            month(2026, Month::October),
        )
        .expect("a revision of the same category is allowed");

    assert_eq!(
        report(&fixture, month(2026, Month::September)).budgeted(),
        pkr(2_000_000),
        "September keeps the limit that was in force"
    );
    assert_eq!(
        report(&fixture, month(2026, Month::October)).budgeted(),
        pkr(2_500_000)
    );
    assert_eq!(
        report(&fixture, month(2026, Month::December)).budgeted(),
        pkr(2_500_000)
    );
}

#[test]
fn a_month_before_the_first_budget_has_none() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.groceries,
            pkr(2_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");

    let july = report(&fixture, month(2026, Month::July));

    assert!(july.lines().is_empty());
    assert_eq!(july.budgeted(), pkr(0));
    assert_eq!(july.surplus(), pkr(0));
}

#[test]
fn a_parent_budget_covers_its_children() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(fixture.food, pkr(2_000_000), month(2026, Month::August))
        .expect("the category is an expense category");

    spend(
        &mut fixture.ledger,
        fixture.bank,
        1,
        fixture.groceries,
        500_000,
        day(Month::August, 3),
    );
    spend(
        &mut fixture.ledger,
        fixture.bank,
        2,
        fixture.restaurants,
        300_000,
        day(Month::August, 4),
    );

    let august = report(&fixture, month(2026, Month::August));
    let line = august.lines().first().expect("one budgeted category");

    assert_eq!(line.spent(), pkr(800_000));
    assert_eq!(line.remaining(), pkr(1_200_000));
    assert_eq!(
        fixture
            .ledger
            .spent_on(fixture.food, month(2026, Month::August))
            .expect("the category is known"),
        pkr(800_000)
    );
}

#[test]
fn spending_is_classified_against_the_limit() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.transport,
            pkr(1_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");

    spend(
        &mut fixture.ledger,
        fixture.bank,
        1,
        fixture.transport,
        800_000,
        day(Month::August, 3),
    );
    assert_eq!(
        report(&fixture, month(2026, Month::August)).lines()[0].state(),
        BudgetState::Within,
        "80 percent is below the threshold"
    );

    spend(
        &mut fixture.ledger,
        fixture.bank,
        2,
        fixture.transport,
        100_000,
        day(Month::August, 4),
    );
    assert_eq!(
        report(&fixture, month(2026, Month::August)).lines()[0].state(),
        BudgetState::NearLimit,
        "exactly 90 percent is near the limit"
    );

    spend(
        &mut fixture.ledger,
        fixture.bank,
        3,
        fixture.transport,
        100_000,
        day(Month::August, 5),
    );
    assert_eq!(
        report(&fixture, month(2026, Month::August)).lines()[0].state(),
        BudgetState::NearLimit,
        "spending the whole budget is not overspending"
    );

    spend(
        &mut fixture.ledger,
        fixture.bank,
        4,
        fixture.transport,
        1,
        day(Month::August, 6),
    );
    assert_eq!(
        report(&fixture, month(2026, Month::August)).lines()[0].state(),
        BudgetState::Over,
        "one minor unit over the limit is over"
    );
}

#[test]
fn the_surplus_is_the_month_end_offer() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.groceries,
            pkr(2_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");
    fixture
        .ledger
        .set_budget(
            fixture.transport,
            pkr(1_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");

    spend(
        &mut fixture.ledger,
        fixture.bank,
        1,
        fixture.groceries,
        1_800_000,
        day(Month::August, 3),
    );
    spend(
        &mut fixture.ledger,
        fixture.bank,
        2,
        fixture.transport,
        950_000,
        day(Month::August, 4),
    );

    let august = report(&fixture, month(2026, Month::August));

    assert_eq!(august.budgeted(), pkr(3_000_000));
    assert_eq!(august.spent(), pkr(2_750_000));
    assert_eq!(august.remaining(), pkr(250_000));
    assert_eq!(august.surplus(), pkr(250_000), "PKR 2,500.00 is offered");
    assert!(!august.is_over_budget());
}

#[test]
fn an_overspent_category_reduces_the_offer_and_never_goes_negative() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.groceries,
            pkr(2_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");
    fixture
        .ledger
        .set_budget(
            fixture.transport,
            pkr(1_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");

    spend(
        &mut fixture.ledger,
        fixture.bank,
        1,
        fixture.groceries,
        2_200_000,
        day(Month::August, 3),
    );
    spend(
        &mut fixture.ledger,
        fixture.bank,
        2,
        fixture.transport,
        950_000,
        day(Month::August, 4),
    );

    let august = report(&fixture, month(2026, Month::August));

    assert_eq!(
        august.remaining(),
        pkr(-150_000),
        "2,000 over on groceries, 500 under on transport"
    );
    assert_eq!(august.surplus(), pkr(0), "the offer floors at zero");
    assert!(august.is_over_budget());
}

#[test]
fn income_and_transfers_do_not_count_against_a_budget() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(fixture.food, pkr(2_000_000), month(2026, Month::August))
        .expect("the category is an expense category");

    let income = Transaction::income(
        TransactionId::from_uuid(Uuid::from_u128(1)),
        day(Month::August, 1),
        fixture.bank,
        fixture.salary,
        pkr(15_000_000),
    )
    .expect("the amount is positive");
    fixture.ledger.record(income).expect("the category matches");

    let transfer = Transaction::transfer(
        TransactionId::from_uuid(Uuid::from_u128(2)),
        day(Month::August, 2),
        fixture.bank,
        fixture.cash,
        pkr(3_000_000),
    )
    .expect("the amount is positive");
    fixture
        .ledger
        .record(transfer)
        .expect("both accounts exist");

    let august = report(&fixture, month(2026, Month::August));

    assert_eq!(august.spent(), pkr(0));
    assert_eq!(august.surplus(), pkr(2_000_000));
}

#[test]
fn a_budget_needs_an_expense_category() {
    let mut fixture = fixture();

    assert_eq!(
        fixture
            .ledger
            .set_budget(fixture.salary, pkr(1_000), month(2026, Month::August)),
        Err(LedgerError::CategoryKindMismatch {
            expected: CategoryKind::Expense,
            found: CategoryKind::Income,
        })
    );
}

#[test]
fn a_budget_needs_a_known_category() {
    let mut fixture = fixture();
    let ghost = category_id(99);

    assert_eq!(
        fixture
            .ledger
            .set_budget(ghost, pkr(1_000), month(2026, Month::August)),
        Err(LedgerError::UnknownCategory(ghost))
    );
}

#[test]
fn a_budget_must_be_positive() {
    let mut fixture = fixture();

    for amount in [pkr(0), pkr(-1_000)] {
        assert_eq!(
            fixture
                .ledger
                .set_budget(fixture.food, amount, month(2026, Month::August)),
            Err(LedgerError::NonPositiveBudget(fixture.food))
        );
    }
}

#[test]
fn a_parent_and_its_child_cannot_both_be_budgeted() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(fixture.food, pkr(2_000_000), month(2026, Month::August))
        .expect("the parent is an expense category");

    assert_eq!(
        fixture
            .ledger
            .set_budget(fixture.groceries, pkr(500_000), month(2026, Month::August)),
        Err(LedgerError::BudgetConflict {
            category: fixture.groceries,
            other: fixture.food,
        }),
        "both would count the same spending"
    );
}

#[test]
fn a_child_budgeted_first_blocks_the_parent() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(fixture.groceries, pkr(500_000), month(2026, Month::August))
        .expect("the child is an expense category");

    assert_eq!(
        fixture
            .ledger
            .set_budget(fixture.food, pkr(2_000_000), month(2026, Month::October)),
        Err(LedgerError::BudgetConflict {
            category: fixture.food,
            other: fixture.groceries,
        }),
        "the conflict is caught even though the months differ"
    );
}

#[test]
fn clearing_a_budget_frees_the_parent() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(fixture.groceries, pkr(500_000), month(2026, Month::August))
        .expect("the child is an expense category");

    assert_eq!(
        fixture.ledger.clear_budget(fixture.groceries),
        Ok(1),
        "one revision removed"
    );
    fixture
        .ledger
        .set_budget(fixture.food, pkr(2_000_000), month(2026, Month::August))
        .expect("the conflict is gone");
}

#[test]
fn a_threshold_above_one_hundred_is_rejected() {
    let fixture = fixture();

    assert_eq!(
        fixture
            .ledger
            .budget_report(month(2026, Month::August), 101),
        Err(LedgerError::InvalidThreshold { percent: 101 })
    );
}

#[test]
fn carrying_budgets_forward_adjusts_them() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.groceries,
            pkr(1_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");

    let copied = fixture
        .ledger
        .carry_budget_forward(
            month(2026, Month::August),
            month(2026, Month::September),
            10,
        )
        .expect("September is after August");

    assert_eq!(copied, 1);
    assert_eq!(
        report(&fixture, month(2026, Month::September)).budgeted(),
        pkr(1_100_000),
        "ten percent more than August"
    );
    assert_eq!(
        report(&fixture, month(2026, Month::August)).budgeted(),
        pkr(1_000_000),
        "August is untouched"
    );
}

#[test]
fn carrying_budgets_backwards_is_rejected() {
    let mut fixture = fixture();

    assert_eq!(
        fixture.ledger.carry_budget_forward(
            month(2026, Month::September),
            month(2026, Month::August),
            0
        ),
        Err(LedgerError::BudgetPeriodOrder {
            from: month(2026, Month::September),
            to: month(2026, Month::August),
        })
    );
}

#[test]
fn the_report_lists_one_line_per_budgeted_category_in_order() {
    let mut fixture = fixture();
    fixture
        .ledger
        .set_budget(
            fixture.transport,
            pkr(1_000_000),
            month(2026, Month::August),
        )
        .expect("the category is an expense category");
    fixture
        .ledger
        .set_budget(fixture.food, pkr(2_000_000), month(2026, Month::August))
        .expect("the category is an expense category");

    let august = report(&fixture, month(2026, Month::August));
    let categories: Vec<CategoryId> = august.lines().iter().map(|line| line.category()).collect();

    assert_eq!(
        categories,
        vec![fixture.food, fixture.transport],
        "ordered by category identifier, not by when the budget was set"
    );
    assert_eq!(august.period(), month(2026, Month::August));
}
