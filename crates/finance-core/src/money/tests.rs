//! Unit tests for money parsing, arithmetic and allocation.

use super::{Currency, Money, Rounding};
use crate::error::MoneyError;

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

#[test]
fn parse_and_format_round_trip() {
    assert_eq!(
        Money::parse("1234.56", Currency::PKR).unwrap(),
        pkr(123_456)
    );
    assert_eq!(Money::parse("-0.05", Currency::PKR).unwrap(), pkr(-5));
    assert_eq!(Money::parse(".5", Currency::PKR).unwrap(), pkr(50));
    assert_eq!(Money::parse("7.", Currency::PKR).unwrap(), pkr(700));
    assert_eq!(Money::parse("+12", Currency::PKR).unwrap(), pkr(1_200));
    assert_eq!(pkr(123_456).to_plain_string(), "1234.56");
    assert_eq!(pkr(-5).to_plain_string(), "-0.05");
    assert_eq!(pkr(0).to_plain_string(), "0.00");
}

#[test]
fn respects_currency_exponent() {
    assert_eq!(Money::parse("500", Currency::JPY).unwrap().minor(), 500);
    assert_eq!(Money::parse("1.234", Currency::KWD).unwrap().minor(), 1_234);
    assert_eq!(Currency::JPY.exponent(), 0);
    assert_eq!(Currency::KWD.exponent(), 3);
    assert_eq!(pkr(0).to_plain_string(), "0.00");
}

#[test]
fn rejects_more_decimals_than_the_currency_allows() {
    let error = Money::parse("1.234", Currency::PKR).unwrap_err();
    assert_eq!(
        error,
        MoneyError::TooManyDecimals {
            currency: Currency::PKR,
            max: 2,
            found: 3
        }
    );
    assert!(Money::parse("1.5", Currency::JPY).is_err());
}

#[test]
fn rejects_garbage() {
    for input in ["", " ", "abc", "1.2.3", "1,000", "--1", "1e5", "١٢٣"] {
        assert!(
            Money::parse(input, Currency::PKR).is_err(),
            "expected {input:?} to be rejected"
        );
    }
}

#[test]
fn addition_requires_one_currency() {
    let error = pkr(100).checked_add(Money::from_minor(100, Currency::USD));
    assert_eq!(
        error,
        Err(MoneyError::CurrencyMismatch {
            left: Currency::PKR,
            right: Currency::USD
        })
    );
}

#[test]
fn division_rounds_as_requested() {
    let amount = pkr(10);
    assert_eq!(amount.div(4, Rounding::HalfUp).unwrap(), pkr(3));
    assert_eq!(amount.div(4, Rounding::HalfEven).unwrap(), pkr(2));
    assert_eq!(amount.div(4, Rounding::Down).unwrap(), pkr(2));
    assert_eq!(amount.div(4, Rounding::Up).unwrap(), pkr(3));

    let negative = pkr(-10);
    assert_eq!(negative.div(4, Rounding::HalfEven).unwrap(), pkr(-2));
    assert_eq!(negative.div(4, Rounding::HalfUp).unwrap(), pkr(-3));
    assert_eq!(
        amount.div(0, Rounding::HalfEven),
        Err(MoneyError::DivisionByZero)
    );
}

#[test]
fn allocation_sums_to_the_total() {
    let parts = pkr(100).allocate(&[1, 1, 1]).unwrap();
    assert_eq!(parts, vec![pkr(34), pkr(33), pkr(33)]);
    let sum = parts
        .iter()
        .fold(pkr(0), |total, part| total.checked_add(*part).unwrap());
    assert_eq!(sum, pkr(100));
}

#[test]
fn allocation_handles_negative_amounts_exactly() {
    let parts = pkr(-100).allocate(&[1, 1, 1]).unwrap();
    assert_eq!(parts, vec![pkr(-34), pkr(-33), pkr(-33)]);
    let sum = parts
        .iter()
        .fold(pkr(0), |total, part| total.checked_add(*part).unwrap());
    assert_eq!(sum, pkr(-100));
}

#[test]
fn allocation_respects_weights() {
    let parts = pkr(1_000).allocate(&[7, 3]).unwrap();
    assert_eq!(parts, vec![pkr(700), pkr(300)]);
}

#[test]
fn allocation_rejects_empty_weights() {
    assert_eq!(pkr(100).allocate(&[]), Err(MoneyError::EmptyAllocation));
    assert_eq!(pkr(100).allocate(&[0, 0]), Err(MoneyError::EmptyAllocation));
}

#[test]
fn overflow_is_an_error_not_a_wrap() {
    let huge = Money::from_minor(i64::MAX, Currency::PKR);
    assert_eq!(huge.checked_add(pkr(1)), Err(MoneyError::Overflow));
}

#[test]
fn display_includes_the_currency_code() {
    assert_eq!(pkr(123_456).to_string(), "1234.56 PKR");
}
