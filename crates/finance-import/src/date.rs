//! Dates, in the formats a bank export actually uses.
//!
//! The format is part of the profile rather than guessed per row. A file where
//! `03/04/2026` sometimes means March and sometimes April cannot be parsed
//! correctly, so the profile commits to day-first and the preview shows the
//! result before anything is saved.

use time::{Date, Month};

/// How the dates in a statement are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateFormat {
    /// `12-Aug-2026`, the format of the confirmed bank profile.
    DayMonthNameYear,
    /// `12-08-2026`, day first.
    DayMonthYearDash,
    /// `12/08/2026`, day first. Not month-first, which would be a different
    /// profile.
    DayMonthYearSlash,
    /// `2026-08-12`, the unambiguous ISO order.
    IsoDate,
}

/// Parses `value` with `format`, or returns `None`.
///
/// A date that does not exist, such as 31 February, is `None`: a bank that
/// printed it produced a file that cannot be trusted for that row.
pub fn parse_date(value: &str, format: DateFormat) -> Option<Date> {
    let text = value.trim();
    match format {
        DateFormat::IsoDate => {
            let [year, month, day] = split3(text, '-')?;
            numeric(year, month, day)
        }
        DateFormat::DayMonthYearDash => {
            let [day, month, year] = split3(text, '-')?;
            numeric(year, month, day)
        }
        DateFormat::DayMonthYearSlash => {
            let [day, month, year] = split3(text, '/')?;
            numeric(year, month, day)
        }
        DateFormat::DayMonthNameYear => {
            let [day, month, year] = split3(text, '-')?;
            let year = i32::try_from(number(year, 4)?).ok()?;
            let day = u8::try_from(number(day, 2)?).ok()?;
            Date::from_calendar_date(year, named_month(month)?, day).ok()
        }
    }
}

fn split3(text: &str, separator: char) -> Option<[&str; 3]> {
    let mut parts = text.split(separator);
    let first = parts.next()?;
    let second = parts.next()?;
    let third = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some([first, second, third])
}

fn numeric(year: &str, month: &str, day: &str) -> Option<Date> {
    let year = i32::try_from(number(year, 4)?).ok()?;
    let month = u8::try_from(number(month, 2)?).ok()?;
    let day = u8::try_from(number(day, 2)?).ok()?;
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

fn number(text: &str, max_digits: usize) -> Option<u32> {
    if text.is_empty() || text.len() > max_digits || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    text.parse().ok()
}

/// Matches the first three letters of an English month name, so `Aug`,
/// `august` and `AUG` all work.
fn named_month(text: &str) -> Option<Month> {
    let upper = text.to_ascii_uppercase();
    let key = upper.get(..3)?;
    match key {
        "JAN" => Some(Month::January),
        "FEB" => Some(Month::February),
        "MAR" => Some(Month::March),
        "APR" => Some(Month::April),
        "MAY" => Some(Month::May),
        "JUN" => Some(Month::June),
        "JUL" => Some(Month::July),
        "AUG" => Some(Month::August),
        "SEP" => Some(Month::September),
        "OCT" => Some(Month::October),
        "NOV" => Some(Month::November),
        "DEC" => Some(Month::December),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
