//! Reading meaning out of a description, without deciding anything.
//!
//! Two hints come from the text: a shortened merchant name for card purchases,
//! and a suspicion that a row moves money between the owner's own accounts.
//! Both are suggestions. The review screen confirms them, and the remembered
//! rule is what applies them next time (FR-033).
//!
//! The full description is never replaced by a hint. It is what the user
//! recognises in the register, and it is what the duplicate check compares.

use finance_core::TransferPattern;

/// The shortened merchant name a card description carries, if any.
///
/// The confirmed shape is `POS SALE - VISA AT <MERCHANT> <CITY> <COUNTRY>`.
/// The merchant is the text after `AT`, without the city and country, so a
/// remembered merchant rule survives a change of branch or city. The country
/// code is the signal that a city precedes it: with no two-letter code at the
/// end, nothing is dropped.
///
/// Returns `None` when there is no `AT` marker, so an ATM line is not mistaken
/// for a shop.
pub fn merchant(description: &str) -> Option<String> {
    let upper = description.to_ascii_uppercase();
    let at = upper.rfind(" AT ")?;
    // Upper-casing ASCII keeps byte offsets, so `at` is valid in `description`.
    let tail = description.get(at + 4..)?.trim();
    if tail.is_empty() {
        return None;
    }

    let mut tokens: Vec<&str> = tail.split_whitespace().collect();
    if tokens.last().is_some_and(|token| is_country_code(token)) {
        tokens.pop();
        // The token before a country code is the city, and only when the
        // merchant still has a name of its own after removing it.
        if tokens.len() > 1 {
            tokens.pop();
        }
    }

    if tokens.is_empty() {
        return None;
    }

    // `SAMPLE CAFE, KARACHI PK` uses a comma to separate merchant from city;
    // the comma is punctuation, not part of the name.
    let name = tokens.join(" ");
    let name = name.trim_end_matches([',', ';']).trim_end();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn is_country_code(token: &str) -> bool {
    token.len() == 2 && token.bytes().all(|byte| byte.is_ascii_uppercase())
}

/// Guesses whether a row moves money between the owner's own accounts.
///
/// Cash withdrawals and peer-to-peer transfers are the two patterns the
/// confirmed statement contains. A `P2P FT FROM` row is money arriving from the
/// holder's other account, so importing it as income would overstate the savings
/// rate by its full amount.
///
/// The answer is the domain's [`TransferPattern`], because a remembered rule is
/// keyed on it (FR-033).
pub fn transfer_hint(description: &str) -> Option<TransferPattern> {
    let upper = description.to_ascii_uppercase();

    if upper.contains("ATM CASH") {
        return Some(TransferPattern::CashWithdrawal);
    }
    if upper.contains("P2P FT") || upper.contains("IBFT") {
        return Some(TransferPattern::peer_transfer(
            counterparty(description).as_deref(),
        ));
    }

    None
}

/// Reads the name out of `... FROM <NAME> <BANK> ACCT: ...`.
///
/// The token immediately before `ACCT` is the counterparty's bank, so it is
/// dropped along with the account number that follows.
fn counterparty(description: &str) -> Option<String> {
    let upper = description.to_ascii_uppercase();
    let from = upper.find("FROM ")?;
    let tail = description.get(from + 5..)?.trim();
    let tail_upper = tail.to_ascii_uppercase();

    let name = match tail_upper.find(" ACCT") {
        Some(index) => {
            let before = tail.get(..index)?;
            let mut tokens: Vec<&str> = before.split_whitespace().collect();
            tokens.pop();
            tokens.join(" ")
        }
        None => tail.split_whitespace().collect::<Vec<_>>().join(" "),
    };

    if name.is_empty() { None } else { Some(name) }
}

#[cfg(test)]
mod tests;
