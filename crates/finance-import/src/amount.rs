//! Amounts as a bank writes them: thousands separators, parentheses for
//! negatives, and an empty cell that means zero.
//!
//! The parser strips presentation and hands the digits to [`Money::parse`], so
//! there is exactly one place in the app that turns decimal text into minor
//! units (ADR-0003).

use finance_core::{Currency, Money};

/// Parses an amount, or returns `None` when it is not a number.
///
/// An empty cell is zero, because a debit/credit export writes `0` or nothing
/// for the column that does not apply. A leading `-`, a leading `+` and
/// enclosing parentheses are accepted; the sign is still checked by the caller,
/// which knows whether the column may be negative.
///
/// Commas are allowed between digits only. Group sizes are not validated: both
/// `1,250,000.00` and `12,50,000.00` are in use locally, and the running-balance
/// check is a stronger guard than a grouping rule would be.
pub fn parse_amount(value: &str, currency: Currency) -> Option<Money> {
    let text = value.trim();
    if text.is_empty() {
        return Some(Money::zero(currency));
    }

    let (mut negative, body) = match text.strip_prefix('(') {
        Some(inner) => (true, inner.strip_suffix(')')?),
        None => (false, text),
    };
    let body = match body.strip_prefix('-') {
        Some(inner) => {
            negative = !negative;
            inner
        }
        None => body,
    };
    let body = body.strip_prefix('+').unwrap_or(body).trim();

    let (whole, fraction) = match body.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (body, ""),
    };
    if !groups(whole) || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let digits = whole.replace(',', "");
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut plain = String::with_capacity(body.len() + 1);
    if negative {
        plain.push('-');
    }
    plain.push_str(&digits);
    if !fraction.is_empty() {
        plain.push('.');
        plain.push_str(fraction);
    }

    Money::parse(&plain, currency).ok()
}

/// True when the integer part has no empty comma group, so `,123` and `1,,000`
/// are refused rather than silently accepted.
fn groups(whole: &str) -> bool {
    whole.split(',').all(|group| !group.is_empty())
}

#[cfg(test)]
mod tests;
