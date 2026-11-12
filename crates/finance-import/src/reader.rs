//! Locating the header and turning records into rows.
//!
//! A bank export usually has title lines above the header, so the header is
//! found by looking for the profile's columns rather than by assuming line 1.
//! Everything after it is a row; a row that cannot be read becomes an issue
//! instead of stopping the parse.

use finance_core::{Currency, Money};

use crate::amount;
use crate::date;
use crate::error::{ImportError, RowIssue, RowProblem};
use crate::profile::{Direction, Profile};
use crate::statement::{Flow, OpeningBalance, Statement, StatementRow};
use crate::text;

/// How many rows may precede the header before the file is judged to be the
/// wrong shape.
pub const MAX_PREAMBLE: usize = 50;

/// Parses a statement.
///
/// `bytes` is the file's contents; this function does no I/O. `currency` is the
/// destination account's currency, which every amount is parsed into.
///
/// # Errors
///
/// Returns [`ImportError::Empty`] for a file with no text,
/// [`ImportError::Encoding`] when the bytes are neither UTF-8 nor UTF-16,
/// [`ImportError::MissingHeader`] when no row in the first
/// [`MAX_PREAMBLE`] matches the profile, [`ImportError::MissingColumn`] and
/// [`ImportError::DuplicateColumn`] for an ambiguous header, and
/// [`ImportError::Overflow`] when the totals do not fit.
///
/// A row that cannot be read is not an error: it is recorded in
/// [`Statement::issues`] with its line number and the rest of the file is still
/// parsed.
pub fn parse(
    bytes: &[u8],
    profile: &Profile,
    currency: Currency,
) -> Result<Statement, ImportError> {
    let contents = text::decode(bytes)?;
    if contents.trim().is_empty() {
        return Err(ImportError::Empty);
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(contents.as_bytes());

    let mut records = reader.records();
    let (header, header_line) = locate_header(&mut records, profile)?;
    let columns = Columns::resolve(&header, profile)?;

    let mut rows = Vec::new();
    let mut opening: Option<OpeningBalance> = None;
    let mut issues = Vec::new();
    let mut data_rows = 0;

    for result in records {
        let record = result.map_err(malformed)?;
        if record.iter().all(str::is_empty) {
            continue;
        }
        data_rows += 1;
        let line = line_of(&record, header_line + data_rows);

        match read_row(&record, line, &columns, profile, currency) {
            Ok(Row::Transaction(row)) => rows.push(row),
            Ok(Row::Opening(candidate)) => {
                if opening.is_some() {
                    issues.push(RowIssue::new(line, RowProblem::RepeatedOpeningBalance));
                } else {
                    opening = Some(candidate);
                }
            }
            Err(problem) => issues.push(RowIssue::new(line, problem)),
        }
    }

    Statement::assemble(currency, header_line, data_rows, rows, opening, issues)
}

/// A row that became something usable.
enum Row {
    Transaction(StatementRow),
    Opening(OpeningBalance),
}

/// The resolved column indices for one file.
struct Columns {
    width: usize,
    date: usize,
    description: usize,
    instrument: Option<usize>,
    balance: Option<usize>,
    direction: ResolvedDirection,
}

/// Where direction comes from, with the column names kept for error messages.
enum ResolvedDirection {
    Signed {
        index: usize,
        column: String,
    },
    DebitCredit {
        debit: usize,
        credit: usize,
        debit_column: String,
        credit_column: String,
    },
}

impl Columns {
    fn resolve(header: &csv::StringRecord, profile: &Profile) -> Result<Self, ImportError> {
        let find = |name: &str| -> Result<usize, ImportError> {
            let mut matches = header
                .iter()
                .enumerate()
                .filter(|(_, cell)| text::same(cell, name));
            let first = matches.next().map(|(index, _)| index).ok_or_else(|| {
                ImportError::MissingColumn {
                    field: name.to_owned(),
                }
            })?;
            if matches.next().is_some() {
                return Err(ImportError::DuplicateColumn {
                    field: name.to_owned(),
                });
            }
            Ok(first)
        };

        let date = find(profile.date_column())?;
        let description = find(profile.description_column())?;
        let instrument = profile.instrument_column().map(find).transpose()?;
        let balance = profile.balance_column().map(find).transpose()?;

        let direction = match profile.direction() {
            Direction::Signed { amount } => ResolvedDirection::Signed {
                index: find(amount)?,
                column: amount.clone(),
            },
            Direction::DebitCredit { debit, credit } => ResolvedDirection::DebitCredit {
                debit: find(debit)?,
                credit: find(credit)?,
                debit_column: debit.clone(),
                credit_column: credit.clone(),
            },
        };

        Ok(Self {
            width: header.len(),
            date,
            description,
            instrument,
            balance,
            direction,
        })
    }
}

fn locate_header<R: std::io::Read>(
    records: &mut csv::StringRecordsIter<'_, R>,
    profile: &Profile,
) -> Result<(csv::StringRecord, usize), ImportError> {
    for (index, result) in records.enumerate() {
        let record = result.map_err(malformed)?;
        let line = line_of(&record, index + 1);

        let is_header = profile
            .required_columns()
            .iter()
            .all(|column| record.iter().any(|cell| text::same(cell, column)));
        if is_header {
            return Ok((record, line));
        }

        if index + 1 >= MAX_PREAMBLE {
            break;
        }
    }

    Err(ImportError::MissingHeader {
        profile: profile.name().to_owned(),
        expected: profile.required_columns().join(", "),
        limit: MAX_PREAMBLE,
    })
}

fn read_row(
    record: &csv::StringRecord,
    line: usize,
    columns: &Columns,
    profile: &Profile,
    currency: Currency,
) -> Result<Row, RowProblem> {
    if record.len() != columns.width {
        return Err(RowProblem::WrongColumnCount {
            expected: columns.width,
            found: record.len(),
        });
    }

    let date_text = cell(record, columns.date);
    let date =
        date::parse_date(date_text, profile.date_format()).ok_or_else(|| RowProblem::Date {
            value: date_text.to_owned(),
        })?;

    let description = cell(record, columns.description).trim();
    if description.is_empty() {
        return Err(RowProblem::NoDescription);
    }
    let description = description.to_owned();

    let instrument = columns
        .instrument
        .map(|index| cell(record, index).trim())
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    let balance = match columns.balance {
        Some(index) => {
            let value = cell(record, index);
            Some(
                amount::parse_amount(value, currency).ok_or_else(|| RowProblem::Balance {
                    value: value.to_owned(),
                })?,
            )
        }
        None => None,
    };

    match read_flow(record, &columns.direction, currency)? {
        Some(flow) => Ok(Row::Transaction(StatementRow::new(
            line,
            date,
            description,
            instrument,
            flow,
            balance,
        ))),
        None => read_opening(line, date, &description, balance),
    }
}

fn read_opening(
    line: usize,
    date: time::Date,
    description: &str,
    balance: Option<Money>,
) -> Result<Row, RowProblem> {
    if !is_opening_marker(description) {
        return Err(RowProblem::NoAmount);
    }
    let Some(amount) = balance else {
        return Err(RowProblem::OpeningBalanceWithoutBalance);
    };
    if !amount.is_positive() {
        return Err(RowProblem::OpeningBalanceNotPositive { amount });
    }

    Ok(Row::Opening(OpeningBalance::new(line, date, amount)))
}

/// True for the marker row the bank writes before the first transaction.
fn is_opening_marker(description: &str) -> bool {
    text::normalise(description).contains("OPENING BALANCE")
}

/// Reads the direction and amount, or `None` when the row moves no money.
fn read_flow(
    record: &csv::StringRecord,
    direction: &ResolvedDirection,
    currency: Currency,
) -> Result<Option<Flow>, RowProblem> {
    match direction {
        ResolvedDirection::Signed { index, column } => {
            let value = cell(record, *index);
            let amount =
                amount::parse_amount(value, currency).ok_or_else(|| RowProblem::Amount {
                    column: column.clone(),
                    value: value.to_owned(),
                })?;

            if amount.is_zero() {
                return Ok(None);
            }
            if amount.is_positive() {
                return Ok(Flow::incoming(amount));
            }

            let magnitude = amount.checked_neg().map_err(|_| RowProblem::Amount {
                column: column.clone(),
                value: value.to_owned(),
            })?;
            Ok(Flow::outgoing(magnitude))
        }
        ResolvedDirection::DebitCredit {
            debit,
            credit,
            debit_column,
            credit_column,
        } => {
            let debit_value = cell(record, *debit);
            let debit_amount =
                amount::parse_amount(debit_value, currency).ok_or_else(|| RowProblem::Amount {
                    column: debit_column.clone(),
                    value: debit_value.to_owned(),
                })?;
            let credit_value = cell(record, *credit);
            let credit_amount =
                amount::parse_amount(credit_value, currency).ok_or_else(|| RowProblem::Amount {
                    column: credit_column.clone(),
                    value: credit_value.to_owned(),
                })?;

            if debit_amount.is_negative() {
                return Err(RowProblem::Negative {
                    column: debit_column.clone(),
                    value: debit_value.to_owned(),
                });
            }
            if credit_amount.is_negative() {
                return Err(RowProblem::Negative {
                    column: credit_column.clone(),
                    value: credit_value.to_owned(),
                });
            }
            if debit_amount.is_positive() && credit_amount.is_positive() {
                return Err(RowProblem::BothDirections {
                    debit: debit_value.to_owned(),
                    credit: credit_value.to_owned(),
                });
            }
            if debit_amount.is_positive() {
                return Ok(Flow::outgoing(debit_amount));
            }
            if credit_amount.is_positive() {
                return Ok(Flow::incoming(credit_amount));
            }
            Ok(None)
        }
    }
}

/// The cell at `index`, or an empty string.
///
/// Every caller has checked the row's width against the header, so the index is
/// in range; returning empty rather than panicking keeps a parser bug from
/// taking the window down with it.
fn cell(record: &csv::StringRecord, index: usize) -> &str {
    record.get(index).unwrap_or_default()
}

fn line_of(record: &csv::StringRecord, fallback: usize) -> usize {
    record
        .position()
        .and_then(|position| usize::try_from(position.line()).ok())
        .unwrap_or(fallback)
}

fn malformed(error: csv::Error) -> ImportError {
    ImportError::Malformed {
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests;
