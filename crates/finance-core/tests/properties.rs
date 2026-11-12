//! Property tests for the invariants that must hold for every input.
//!
//! Unit tests check the examples we thought of. These check the ones we did not:
//! any amount, any split, any mix of transactions.

use finance_core::{
    Account, AccountId, AccountKind, Category, CategoryId, CategoryKind, Currency, Ledger, Money,
    Rounding, Transaction, TransactionId, YearMonth,
};
use proptest::prelude::*;
use time::{Date, Month};
use uuid::Uuid;

const NOVEMBER: Month = Month::November;

fn november_day(number: u8) -> Date {
    Date::from_calendar_date(2026, NOVEMBER, number).expect("a valid November date")
}

proptest! {
    /// A formatted amount must parse back to exactly the same value. This is the
    /// round trip the storage layer and the CSV export both rely on.
    #[test]
    fn a_plain_string_round_trips(minor in -1_000_000_000_i64..=1_000_000_000_i64) {
        let amount = Money::from_minor(minor, Currency::PKR);
        let text = amount.to_plain_string();

        prop_assert_eq!(Money::parse(&text, Currency::PKR), Ok(amount));
    }

    /// A formatted PKR amount always shows exactly two decimal places.
    #[test]
    fn a_plain_string_shows_the_currency_exponent(
        minor in -1_000_000_000_i64..=1_000_000_000_i64
    ) {
        let text = Money::from_minor(minor, Currency::PKR).to_plain_string();
        let (_, fraction) = text.split_once('.').expect("PKR always has a decimal point");

        prop_assert_eq!(fraction.len(), usize::from(Currency::PKR.exponent()));
        prop_assert!(fraction.bytes().all(|byte| byte.is_ascii_digit()));
    }

    /// Splitting an amount can never lose or invent a minor unit, whatever the
    /// weights are. This is what keeps splits and budget allocations honest.
    #[test]
    fn allocation_sums_back_to_the_total(
        minor in -1_000_000_000_i64..=1_000_000_000_i64,
        weights in prop::collection::vec(1_u32..1_000, 1..12),
    ) {
        let amount = Money::from_minor(minor, Currency::PKR);
        let parts = amount.allocate(&weights).expect("the weights are usable");

        prop_assert_eq!(parts.len(), weights.len());

        let sum: i128 = parts.iter().map(|part| i128::from(part.minor())).sum();
        prop_assert_eq!(sum, i128::from(minor));

        // No part may point the opposite way from the amount it came from.
        if minor >= 0 {
            prop_assert!(parts.iter().all(|part| part.minor() >= 0));
        } else {
            prop_assert!(parts.iter().all(|part| part.minor() <= 0));
        }
    }

    /// Rounding may move a quotient by at most one minor unit, and the two
    /// directions must bracket the exact value when multiplied back out.
    #[test]
    fn division_rounds_to_a_quotient_that_brackets_the_exact_value(
        minor in -1_000_000_i64..=1_000_000_i64,
        divisor in 1_i64..1_000,
    ) {
        let amount = Money::from_minor(minor, Currency::PKR);

        let down = amount.div(divisor, Rounding::Down).expect("the divisor is not zero");
        let up = amount.div(divisor, Rounding::Up).expect("the divisor is not zero");
        let nearest = amount.div(divisor, Rounding::HalfUp).expect("the divisor is not zero");

        prop_assert!((up.minor() - down.minor()).abs() <= 1);

        // Down rounds toward zero and Up rounds away from it, so for a negative
        // amount the two are ordered the other way round. Compare the scaled
        // values, which hold whichever sign the amount has.
        let low = down.minor() * divisor;
        let high = up.minor() * divisor;
        prop_assert!(
            low.min(high) <= minor && minor <= low.max(high),
            "{} / {} rounds to {}..{}, which does not bracket it",
            minor,
            divisor,
            low,
            high
        );

        prop_assert!(nearest == down || nearest == up);
    }

    /// The month summary is exactly the sum of the transactions in it, and the
    /// account balance agrees with it. Two independent paths to the same number.
    #[test]
    fn a_month_summary_equals_the_sum_of_its_transactions(
        entries in prop::collection::vec((any::<bool>(), 1_i64..1_000_000), 0..40)
    ) {
        let bank = AccountId::from_uuid(Uuid::from_u128(1));
        let food = CategoryId::from_uuid(Uuid::from_u128(1));
        let salary = CategoryId::from_uuid(Uuid::from_u128(2));

        let mut ledger = Ledger::new(Currency::PKR);
        ledger
            .add_account(Account::new(bank, "Bank", AccountKind::Bank, Currency::PKR))
            .expect("the account is unique");
        ledger
            .add_category(Category::new(food, "Food", CategoryKind::Expense))
            .expect("the category is unique");
        ledger
            .add_category(Category::new(salary, "Salary", CategoryKind::Income))
            .expect("the category is unique");

        let mut expected_income = 0_i64;
        let mut expected_expenses = 0_i64;

        for (index, (is_income, minor)) in entries.iter().enumerate() {
            let amount = Money::from_minor(*minor, Currency::PKR);
            let date = november_day(1 + (index % 30) as u8);
            let id = TransactionId::from_uuid(Uuid::from_u128(index as u128 + 1));

            let transaction = if *is_income {
                expected_income += minor;
                Transaction::income(id, date, bank, salary, amount)
            } else {
                expected_expenses += minor;
                Transaction::expense(id, date, bank, food, amount)
            }
            .expect("the amount is positive");

            ledger.record(transaction).expect("the references are valid");
        }

        let summary = ledger
            .summary(YearMonth::new(2026, NOVEMBER))
            .expect("totals fit");

        prop_assert_eq!(summary.income(), Money::from_minor(expected_income, Currency::PKR));
        prop_assert_eq!(summary.expenses(), Money::from_minor(expected_expenses, Currency::PKR));
        prop_assert_eq!(
            summary.saved(),
            Money::from_minor(expected_income - expected_expenses, Currency::PKR)
        );

        let balance = ledger
            .balance_as_of(bank, november_day(30))
            .expect("the account is known");

        prop_assert_eq!(
            balance,
            Money::from_minor(expected_income - expected_expenses, Currency::PKR)
        );
    }
}
