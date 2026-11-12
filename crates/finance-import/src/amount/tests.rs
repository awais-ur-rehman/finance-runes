use finance_core::Currency;

use super::parse_amount;

fn minor(text: &str) -> Option<i64> {
    parse_amount(text, Currency::PKR).map(finance_core::Money::minor)
}

#[test]
fn reads_thousands_separators_and_two_decimals() {
    assert_eq!(minor("4,500.00"), Some(450_000));
    assert_eq!(minor(" 1,250,000.50 "), Some(125_000_050));
}

#[test]
fn accepts_south_asian_grouping_too() {
    assert_eq!(
        minor("12,50,000.00"),
        Some(125_000_000),
        "lakh grouping is in local use; the balance check is the real guard"
    );
}

#[test]
fn an_empty_cell_is_zero() {
    assert_eq!(minor(""), Some(0));
    assert_eq!(minor("   "), Some(0));
}

#[test]
fn reads_parenthesised_negatives() {
    assert_eq!(minor("(1,200.50)"), Some(-120_050));
    assert_eq!(minor("-840.00"), Some(-84_000));
}

#[test]
fn reads_a_leading_plus() {
    assert_eq!(minor("+250.00"), Some(25_000));
}

#[test]
fn refuses_text_that_is_not_a_number() {
    assert_eq!(minor("abc"), None);
    assert_eq!(minor("1 000.00"), None);
    assert_eq!(minor("--5.00"), None);
}

#[test]
fn refuses_an_empty_comma_group() {
    assert_eq!(minor(",123.00"), None);
    assert_eq!(minor("1,,000.00"), None);
}

#[test]
fn refuses_more_decimals_than_the_currency_has() {
    assert_eq!(
        minor("1.234"),
        None,
        "PKR has two decimal places, so a third is a parse failure, not a rounding decision"
    );
}
