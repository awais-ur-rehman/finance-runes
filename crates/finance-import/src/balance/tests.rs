use finance_core::{Currency, Money};
use time::{Date, Month};

use super::check;
use crate::statement::{Flow, OpeningBalance, StatementRow};

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::August, day).expect("the test date is valid")
}

fn row(line: usize, flow: Flow, balance: i64) -> StatementRow {
    StatementRow::new(
        line,
        date(12),
        "A synthetic row".to_owned(),
        None,
        flow,
        Some(pkr(balance)),
    )
}

fn out(minor: i64) -> Flow {
    Flow::outgoing(pkr(minor)).expect("the test amount is positive")
}

fn incoming(minor: i64) -> Flow {
    Flow::incoming(pkr(minor)).expect("the test amount is positive")
}

#[test]
fn a_consistent_file_has_no_breaks() {
    let opening = OpeningBalance::new(2, date(12), pkr(125_000));
    let rows = vec![
        row(3, incoming(500_000), 625_000),
        row(4, out(84_000), 541_000),
        row(5, out(120_050), 420_950),
    ];

    assert!(check(Some(&opening), &rows, &[]).is_empty());
}

#[test]
fn a_break_names_the_line_and_both_balances() {
    let opening = OpeningBalance::new(2, date(12), pkr(125_000));
    let rows = vec![
        row(3, incoming(500_000), 625_000),
        row(4, out(84_000), 500_000),
    ];

    let breaks = check(Some(&opening), &rows, &[]);

    assert_eq!(breaks.len(), 1);
    assert_eq!(breaks[0].line(), 4);
    assert_eq!(breaks[0].expected(), pkr(541_000));
    assert_eq!(breaks[0].found(), pkr(500_000));
}

#[test]
fn one_bad_amount_produces_one_break_not_a_cascade() {
    let opening = OpeningBalance::new(2, date(12), pkr(125_000));
    let rows = vec![
        row(3, incoming(500_000), 625_000),
        // The amount says 900 but the balance still follows from 840.
        row(4, out(90_000), 541_000),
        row(5, out(120_050), 420_950),
    ];

    let breaks = check(Some(&opening), &rows, &[]);

    assert_eq!(breaks.len(), 1, "the check resynchronises after a break");
    assert_eq!(breaks[0].line(), 4);
}

#[test]
fn the_first_row_anchors_the_check_when_there_is_no_opening_balance() {
    let rows = vec![
        row(2, incoming(500_000), 625_000),
        row(3, out(84_000), 541_000),
    ];

    assert!(check(None, &rows, &[]).is_empty());
}

#[test]
fn a_row_that_could_not_be_parsed_ends_the_chain() {
    let opening = OpeningBalance::new(2, date(12), pkr(125_000));
    let rows = vec![
        row(3, incoming(500_000), 625_000),
        // Line 4 was dropped as unreadable, so its 840.00 is not in the sum.
        row(5, out(120_050), 420_950),
    ];

    assert!(
        check(Some(&opening), &rows, &[4]).is_empty(),
        "the dropped row explains the arithmetic; the issue already names it"
    );
}

#[test]
fn a_row_without_a_balance_stops_the_check() {
    let opening = OpeningBalance::new(2, date(12), pkr(125_000));
    let rows = vec![
        StatementRow::new(
            3,
            date(12),
            "No balance here".to_owned(),
            None,
            incoming(500_000),
            None,
        ),
        row(4, out(84_000), 541_000),
    ];

    assert!(
        check(Some(&opening), &rows, &[]).is_empty(),
        "nothing after an unreadable balance can be verified, and a guess would be a false alarm"
    );
}
