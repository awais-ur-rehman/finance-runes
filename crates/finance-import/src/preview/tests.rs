use finance_core::{Currency, Money};
use time::{Date, Month};

use crate::error::{RowIssue, RowProblem};
use crate::statement::{OpeningBalance, Statement};

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::August, day).expect("the test date is valid")
}

#[test]
fn a_statement_with_only_an_opening_balance_reports_it_as_the_final_balance() {
    let opening = OpeningBalance::new(2, date(12), pkr(125_000));
    let statement = Statement::assemble(
        Currency::PKR,
        "test".to_owned(),
        1,
        1,
        Vec::new(),
        Some(opening),
        Vec::new(),
    )
    .expect("there are no totals to overflow");

    let preview = statement.preview();

    assert_eq!(preview.rows(), 1);
    assert_eq!(preview.transactions(), 0);
    assert_eq!(preview.final_balance(), Some(pkr(125_000)));
    assert!(preview.is_clean());
}

#[test]
fn an_issue_makes_the_preview_unclean() {
    let issues = vec![RowIssue::new(3, RowProblem::NoAmount)];
    let statement = Statement::assemble(
        Currency::PKR,
        "test".to_owned(),
        1,
        1,
        Vec::new(),
        None,
        issues,
    )
    .expect("no overflow");

    let preview = statement.preview();

    assert!(!preview.is_clean());
    assert_eq!(preview.issues().len(), 1);
    assert_eq!(preview.issues()[0].line(), 3);
}
