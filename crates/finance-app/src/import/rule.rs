//! Matching a row against the rules the ledger remembers.
//!
//! FR-033 asks the app to remember "merchant → category" and "pattern →
//! transfer" and apply both to later rows. The rule itself lives in the domain
//! (`finance_core::Rule`) because it is saved with the ledger; what belongs here
//! is the importer's knowledge of which rows a key matches.
//!
//! A rule is matched against the *derived* value, so a card row whose city or
//! formatting changed still matches its merchant rule.

use finance_core::{Rule, RuleKey, RuleTarget};
use finance_import::StatementRow;

use crate::import::decision::Entry;

/// The rule a row teaches when the user files it, if it teaches one.
///
/// A transfer is remembered as its pattern. Anything else is remembered by its
/// merchant name, and a row with neither teaches nothing: guessing a pattern
/// from free text is how a single correction turns into forty wrong ones.
#[must_use]
pub fn learned_from(row: &StatementRow, entry: Entry) -> Option<Rule> {
    if let (Entry::Transfer(account), Some(pattern)) = (entry, row.transfer_hint()) {
        return Some(Rule::new(
            RuleKey::transfer(pattern),
            RuleTarget::Account(account),
        ));
    }

    let key = RuleKey::merchant(&row.merchant()?)?;
    Some(Rule::new(key, RuleTarget::from(entry)))
}

/// The entry this rule gives a row, when it matches.
///
/// Transfer patterns are checked by the caller before merchant keys, so a
/// merchant rule can never override a recognised own-account movement.
#[must_use]
pub fn apply(rule: &Rule, row: &StatementRow) -> Option<Entry> {
    match rule.key() {
        RuleKey::Merchant(name) => row
            .merchant()
            .filter(|found| same_merchant(found, name))
            .map(|_| Entry::from(rule.target())),
        RuleKey::Transfer(pattern) => row
            .transfer_hint()
            .filter(|found| found == pattern)
            .map(|_| Entry::from(rule.target())),
    }
}

/// Compares merchant names the way the importer reads them: trimmed, ignoring
/// ASCII case. Spacing is already canonical, because the hint is built from
/// whitespace-separated tokens.
fn same_merchant(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}
