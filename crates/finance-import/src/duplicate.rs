//! Recognising a row that has already been imported (FR-035).
//!
//! Two rows are the same transaction when they share a reference number, or —
//! when the bank gives none — the same date, amount and description. The
//! reference is checked first because it is the only key that survives a bank
//! changing how it formats a description.
//!
//! The fallback key normalises the description, because a re-export of the same
//! period sometimes changes spacing or case. Normalising can only merge rows
//! that a person would call identical: same day, same amount, same text.

use std::collections::HashMap;

use finance_core::Money;
use time::Date;

use crate::statement::StatementRow;
use crate::text;

/// Where a reference number came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// The instrument-number column.
    Instrument,
    /// A bank message id embedded in the description.
    MessageId,
}

/// What makes two rows the same transaction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DuplicateKey {
    /// A reference number the bank supplied.
    Reference {
        /// Which field the number came from.
        kind: ReferenceKind,
        /// The number, upper-cased and trimmed.
        value: String,
    },
    /// The best available fallback: day, signed amount and normalised text.
    Fallback {
        /// The posting date.
        date: Date,
        /// The signed amount: negative out, positive in.
        amount: Money,
        /// The description, normalised for comparison.
        description: String,
    },
}

impl DuplicateKey {
    /// The key for a parsed row.
    pub fn for_row(row: &StatementRow) -> Self {
        Self::from_parts(
            row.date(),
            row.flow().signed(),
            row.description(),
            row.instrument(),
        )
    }

    /// The key for an existing transaction, given what the import stored.
    ///
    /// Imported transactions keep the raw description as their note, so a
    /// re-import can be checked against them with the same rules.
    pub fn from_parts(
        date: Date,
        amount: Money,
        description: &str,
        instrument: Option<&str>,
    ) -> Self {
        reference(instrument, description).unwrap_or_else(|| Self::Fallback {
            date,
            amount,
            description: text::normalise(description),
        })
    }
}

/// A row whose key was already used by an earlier row in the same file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Duplicate {
    line: usize,
    first_line: usize,
}

impl Duplicate {
    /// The later line.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// The earlier line it repeats.
    pub const fn first_line(&self) -> usize {
        self.first_line
    }
}

/// Returns every row that repeats an earlier row's key, in file order.
pub(crate) fn within(rows: &[StatementRow]) -> Vec<Duplicate> {
    let mut seen: HashMap<DuplicateKey, usize> = HashMap::new();
    let mut duplicates = Vec::new();

    for row in rows {
        let key = row.duplicate_key();
        match seen.get(&key) {
            Some(first_line) => duplicates.push(Duplicate {
                line: row.line(),
                first_line: *first_line,
            }),
            None => {
                seen.insert(key, row.line());
            }
        }
    }

    duplicates
}

/// The reference key for a row, when the bank supplied one.
fn reference(instrument: Option<&str>, description: &str) -> Option<DuplicateKey> {
    if let Some(instrument) = instrument.map(str::trim).filter(|text| !text.is_empty()) {
        return Some(DuplicateKey::Reference {
            kind: ReferenceKind::Instrument,
            value: text::normalise(instrument),
        });
    }

    message_id(description).map(|value| DuplicateKey::Reference {
        kind: ReferenceKind::MessageId,
        value,
    })
}

/// Reads the value after `MSGID:` in a description.
///
/// The confirmed shape embeds `MSGID: MSG000...` in peer-to-peer rows. The
/// value ends at whitespace, a comma or a semicolon, and a trailing full stop is
/// punctuation rather than part of the id.
fn message_id(description: &str) -> Option<String> {
    const MARKER: &str = "MSGID:";

    let upper = description.to_ascii_uppercase();
    let marker = upper.find(MARKER)?;
    let tail = description.get(marker + MARKER.len()..)?;

    let value: String = tail
        .trim_start()
        .chars()
        .take_while(|character| {
            !character.is_whitespace() && *character != ',' && *character != ';'
        })
        .collect();
    let value = value.trim_end_matches('.').trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_ascii_uppercase())
    }
}

#[cfg(test)]
mod tests;
