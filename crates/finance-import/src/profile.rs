//! The mapping between a bank's columns and the fields the importer needs.
//!
//! A profile is a saved answer to "which column is what" (FR-031). It names
//! columns by header text rather than by position, because the header row is
//! located inside the file and a bank may add a column without moving the ones
//! this profile uses.

use crate::date::DateFormat;

/// Where the direction of a movement comes from.
///
/// The two shapes are genuinely different, so they are an enum rather than two
/// optional column names: a file has one signed amount column *or* a debit and a
/// credit column, never both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    /// One column, signed: negative is money out.
    Signed {
        /// The column's header.
        amount: String,
    },
    /// Two columns, each non-negative; exactly one is non-zero per row.
    DebitCredit {
        /// The money-out column's header.
        debit: String,
        /// The money-in column's header.
        credit: String,
    },
}

/// A saved bank layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    name: String,
    date: String,
    description: String,
    instrument: Option<String>,
    balance: Option<String>,
    direction: Direction,
    date_format: DateFormat,
}

impl Profile {
    /// Builds a profile with no instrument or balance column and
    /// [`DateFormat::DayMonthNameYear`] dates.
    ///
    /// # Panics
    ///
    /// Does not panic; the arguments are only stored. An empty name is caught by
    /// [`Profile::is_complete`] rather than here, so a half-built profile can
    /// still be shown in the editor.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        date: impl Into<String>,
        description: impl Into<String>,
        direction: Direction,
    ) -> Self {
        Self {
            name: name.into(),
            date: date.into(),
            description: description.into(),
            instrument: None,
            balance: None,
            direction,
            date_format: DateFormat::DayMonthNameYear,
        }
    }

    /// The confirmed profile for the owner's bank statement (FR-031).
    ///
    /// Six columns, dates as `12-Aug-2026`, separate debit and credit, an
    /// instrument number and a running balance. See
    /// `docs/requirements/csv-import-profile.md`.
    #[must_use]
    pub fn bank_statement_debit_credit() -> Self {
        Self::new(
            "bank-statement-debit-credit",
            "DATE",
            "PARTICULARS",
            Direction::DebitCredit {
                debit: "DEBIT".to_owned(),
                credit: "CREDIT".to_owned(),
            },
        )
        .with_instrument("INST. NO.")
        .with_balance("BALANCE")
    }

    /// Names the optional instrument-number column.
    #[must_use]
    pub fn with_instrument(mut self, column: impl Into<String>) -> Self {
        self.instrument = Some(column.into());
        self
    }

    /// Names the optional running-balance column.
    #[must_use]
    pub fn with_balance(mut self, column: impl Into<String>) -> Self {
        self.balance = Some(column.into());
        self
    }

    /// Sets how dates are written.
    #[must_use]
    pub const fn with_date_format(mut self, format: DateFormat) -> Self {
        self.date_format = format;
        self
    }

    /// The profile's name, used as the key when it is saved.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The date column's header.
    pub fn date_column(&self) -> &str {
        &self.date
    }

    /// The description column's header.
    pub fn description_column(&self) -> &str {
        &self.description
    }

    /// The instrument-number column, when the bank has one.
    pub fn instrument_column(&self) -> Option<&str> {
        self.instrument.as_deref()
    }

    /// The running-balance column, when the bank has one.
    pub fn balance_column(&self) -> Option<&str> {
        self.balance.as_deref()
    }

    /// Where direction comes from.
    pub const fn direction(&self) -> &Direction {
        &self.direction
    }

    /// How dates are written.
    pub const fn date_format(&self) -> DateFormat {
        self.date_format
    }

    /// The columns a file must have for this profile to read it.
    ///
    /// Used to locate the header row: a row that contains all of these is the
    /// header, wherever it sits below the bank's title lines.
    pub fn required_columns(&self) -> Vec<&str> {
        let mut columns = vec![self.date.as_str(), self.description.as_str()];
        match &self.direction {
            Direction::Signed { amount } => columns.push(amount),
            Direction::DebitCredit { debit, credit } => {
                columns.push(debit);
                columns.push(credit);
            }
        }
        columns
    }

    /// Every column the profile uses, required and optional.
    pub fn columns(&self) -> Vec<&str> {
        let mut columns = self.required_columns();
        columns.extend(self.instrument.as_deref());
        columns.extend(self.balance.as_deref());
        columns
    }
}

#[cfg(test)]
mod tests;
