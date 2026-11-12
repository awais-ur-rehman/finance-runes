use finance_core::{Currency, Money};
use time::{Date, Month};

use super::{DuplicateKey, ReferenceKind, within};
use crate::statement::{Flow, StatementRow};

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::August, day).expect("the test date is valid")
}

fn row(line: usize, instrument: Option<&str>, description: &str, amount: i64) -> StatementRow {
    StatementRow::new(
        line,
        date(12),
        description.to_owned(),
        instrument.map(str::to_owned),
        Flow::outgoing(pkr(amount)).expect("the test amount is positive"),
        None,
    )
}

#[test]
fn an_instrument_number_is_the_key() {
    let key = DuplicateKey::from_parts(date(12), pkr(-84_000), "Anything", Some(" 123456 "));

    assert_eq!(
        key,
        DuplicateKey::Reference {
            kind: ReferenceKind::Instrument,
            value: "123456".to_owned(),
        }
    );
}

#[test]
fn an_empty_instrument_falls_back_to_the_message_id() {
    let key = DuplicateKey::from_parts(
        date(12),
        pkr(500_000),
        "RA AST P2P FT FROM SAMPLE HOLDER. **103 MSGID: msg0001.",
        Some("   "),
    );

    assert_eq!(
        key,
        DuplicateKey::Reference {
            kind: ReferenceKind::MessageId,
            value: "MSG0001".to_owned(),
        },
        "the trailing full stop is punctuation, and case is normalised"
    );
}

#[test]
fn with_no_reference_the_key_is_date_amount_and_text() {
    let key = DuplicateKey::from_parts(
        date(12),
        pkr(-84_000),
        "POS SALE - VISA AT  SAMPLE CAFE",
        None,
    );

    assert_eq!(
        key,
        DuplicateKey::Fallback {
            date: date(12),
            amount: pkr(-84_000),
            description: "POS SALE - VISA AT SAMPLE CAFE".to_owned(),
        },
        "spacing is normalised so a re-export still matches"
    );
}

#[test]
fn a_different_amount_is_a_different_row() {
    let first = DuplicateKey::from_parts(date(12), pkr(-84_000), "SAME TEXT", None);
    let second = DuplicateKey::from_parts(date(12), pkr(-85_000), "SAME TEXT", None);

    assert_ne!(first, second);
}

#[test]
fn a_different_direction_is_a_different_row() {
    let out = DuplicateKey::from_parts(date(12), pkr(-84_000), "SAME TEXT", None);
    let incoming = DuplicateKey::from_parts(date(12), pkr(84_000), "SAME TEXT", None);

    assert_ne!(out, incoming);
}

#[test]
fn a_repeated_row_points_at_the_first_one() {
    let rows = vec![
        row(2, None, "POS SALE - VISA AT SAMPLE CAFE KARACHI PK", 84_000),
        row(3, None, "ATM CASH WITHDRAWAL SAMPLE BRANCH", 200_000),
        row(4, None, "POS SALE - VISA AT SAMPLE CAFE KARACHI PK", 84_000),
    ];

    let duplicates = within(&rows);

    assert_eq!(duplicates.len(), 1);
    assert_eq!(duplicates[0].line(), 4);
    assert_eq!(duplicates[0].first_line(), 2);
}

#[test]
fn rows_with_reference_numbers_are_matched_by_them() {
    let rows = vec![
        row(2, Some("9001"), "CHEQUE ONE", 84_000),
        row(3, Some("9001"), "CHEQUE ONE, CLEARED", 84_000),
    ];

    assert_eq!(
        within(&rows).len(),
        1,
        "the same instrument is the same transaction even if the text differs"
    );
}
