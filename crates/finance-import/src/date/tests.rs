use time::{Date, Month};

use super::{DateFormat, parse_date};

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).expect("the test date is valid")
}

#[test]
fn reads_the_confirmed_bank_format() {
    assert_eq!(
        parse_date("12-Aug-2026", DateFormat::DayMonthNameYear),
        Some(date(2026, Month::August, 12))
    );
    assert_eq!(
        parse_date(" 1-jan-2027 ", DateFormat::DayMonthNameYear),
        Some(date(2027, Month::January, 1)),
        "surrounding space and lower case are accepted"
    );
}

#[test]
fn reads_a_full_month_name() {
    assert_eq!(
        parse_date("12-August-2026", DateFormat::DayMonthNameYear),
        Some(date(2026, Month::August, 12))
    );
}

#[test]
fn refuses_a_day_that_does_not_exist() {
    assert_eq!(
        parse_date("31-Feb-2026", DateFormat::DayMonthNameYear),
        None,
        "the bank printed a date that cannot be trusted"
    );
}

#[test]
fn refuses_an_unknown_month() {
    assert_eq!(
        parse_date("12-Avg-2026", DateFormat::DayMonthNameYear),
        None
    );
}

#[test]
fn refuses_trailing_text() {
    assert_eq!(
        parse_date("12-Aug-2026 00:00", DateFormat::DayMonthNameYear),
        None
    );
}

#[test]
fn reads_iso_dates() {
    assert_eq!(
        parse_date("2026-08-12", DateFormat::IsoDate),
        Some(date(2026, Month::August, 12))
    );
}

#[test]
fn reads_day_first_numeric_dates() {
    assert_eq!(
        parse_date("12/08/2026", DateFormat::DayMonthYearSlash),
        Some(date(2026, Month::August, 12))
    );
    assert_eq!(
        parse_date("12-08-2026", DateFormat::DayMonthYearDash),
        Some(date(2026, Month::August, 12))
    );
}

#[test]
fn a_numeric_date_is_day_first() {
    assert_eq!(
        parse_date("08/12/2026", DateFormat::DayMonthYearSlash),
        Some(date(2026, Month::December, 8)),
        "the profile commits to day-first; month-first would need its own profile"
    );
}
