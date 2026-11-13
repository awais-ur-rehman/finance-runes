//! Row values to domain values, and back.
//!
//! Every function that reads is fallible and names its table in the error, so a
//! corrupt file says which table to look at. Enum text is written through
//! `Display` and read through an explicit match: adding a variant without
//! teaching this module about it is a compile error, not a silent default.

use finance_core::{
    AccountKind, CategoryKind, Currency, PostingTarget, TransactionKind, TransactionSource,
    YearMonth,
};
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

use crate::StoreError;

fn corrupt(table: &'static str, detail: impl Into<String>) -> StoreError {
    StoreError::Corrupt {
        table,
        detail: detail.into(),
    }
}

/// A UUID as the hyphenated text SQLite stores.
pub(crate) fn uuid(value: Uuid) -> String {
    value.to_string()
}

/// Parses a stored identifier.
pub(crate) fn parse_uuid(text: &str, table: &'static str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(text)
        .map_err(|error| corrupt(table, format!("identifier {text:?} is not a UUID: {error}")))
}

/// A date as `YYYY-MM-DD`, which sorts correctly as text and reads without a
/// client.
pub(crate) fn date(value: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        value.year(),
        u8::from(value.month()),
        value.day()
    )
}

/// Parses a stored `YYYY-MM-DD` date.
pub(crate) fn parse_date(text: &str, table: &'static str) -> Result<Date, StoreError> {
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(corrupt(table, format!("date {text:?} is not YYYY-MM-DD")));
    };

    let year: i32 = year
        .parse()
        .map_err(|_| corrupt(table, format!("date {text:?} has an invalid year")))?;
    let month: u8 = month
        .parse()
        .map_err(|_| corrupt(table, format!("date {text:?} has an invalid month")))?;
    let day: u8 = day
        .parse()
        .map_err(|_| corrupt(table, format!("date {text:?} has an invalid day")))?;

    let month = Month::try_from(month)
        .map_err(|_| corrupt(table, format!("date {text:?} has month {month}")))?;
    Date::from_calendar_date(year, month, day)
        .map_err(|error| corrupt(table, format!("date {text:?} is not a real day: {error}")))
}

/// A budget period as `YYYY-MM`.
pub(crate) fn period(value: YearMonth) -> String {
    value.to_string()
}

/// Parses a stored `YYYY-MM` budget period.
pub(crate) fn parse_period(text: &str, table: &'static str) -> Result<YearMonth, StoreError> {
    let Some((year, month)) = text.split_once('-') else {
        return Err(corrupt(table, format!("period {text:?} is not YYYY-MM")));
    };

    let year: i32 = year
        .parse()
        .map_err(|_| corrupt(table, format!("period {text:?} has an invalid year")))?;
    let month: u8 = month
        .parse()
        .map_err(|_| corrupt(table, format!("period {text:?} has an invalid month")))?;
    let month = Month::try_from(month)
        .map_err(|_| corrupt(table, format!("period {text:?} has month {month}")))?;

    Ok(YearMonth::new(year, month))
}

/// A timestamp as RFC 3339 text, readable in any SQLite client.
pub(crate) fn timestamp(value: OffsetDateTime) -> Result<String, StoreError> {
    use time::format_description::well_known::Rfc3339;

    value.format(&Rfc3339).map_err(|error| {
        corrupt(
            "schema_migrations",
            format!("cannot format a time: {error}"),
        )
    })
}

/// A currency from its stored code and exponent.
pub(crate) fn parse_currency(
    code: &str,
    exponent: i64,
    table: &'static str,
) -> Result<Currency, StoreError> {
    let exponent = u8::try_from(exponent).map_err(|_| {
        corrupt(
            table,
            format!("currency exponent {exponent} is out of range"),
        )
    })?;
    Currency::new(code, exponent).map_err(|error| corrupt(table, error.to_string()))
}

/// An account kind's stored text.
pub(crate) fn account_kind(value: AccountKind) -> String {
    value.to_string()
}

/// Parses a stored account kind.
pub(crate) fn parse_account_kind(
    text: &str,
    table: &'static str,
) -> Result<AccountKind, StoreError> {
    match text {
        "bank" => Ok(AccountKind::Bank),
        "cash" => Ok(AccountKind::Cash),
        "savings" => Ok(AccountKind::Savings),
        "card" => Ok(AccountKind::Card),
        "wallet" => Ok(AccountKind::Wallet),
        other => Err(corrupt(table, format!("unknown account kind {other:?}"))),
    }
}

/// A category kind's stored text.
pub(crate) fn category_kind(value: CategoryKind) -> String {
    value.to_string()
}

/// Parses a stored category kind.
pub(crate) fn parse_category_kind(
    text: &str,
    table: &'static str,
) -> Result<CategoryKind, StoreError> {
    match text {
        "expense" => Ok(CategoryKind::Expense),
        "income" => Ok(CategoryKind::Income),
        other => Err(corrupt(table, format!("unknown category kind {other:?}"))),
    }
}

/// A transaction kind's stored text.
pub(crate) fn transaction_kind(value: TransactionKind) -> String {
    value.to_string()
}

/// Parses a stored transaction kind.
pub(crate) fn parse_transaction_kind(
    text: &str,
    table: &'static str,
) -> Result<TransactionKind, StoreError> {
    match text {
        "expense" => Ok(TransactionKind::Expense),
        "income" => Ok(TransactionKind::Income),
        "transfer" => Ok(TransactionKind::Transfer),
        "opening" => Ok(TransactionKind::Opening),
        other => Err(corrupt(
            table,
            format!("unknown transaction kind {other:?}"),
        )),
    }
}

/// A transaction source as its two columns: the kind, and the profile when the
/// row came from an import.
pub(crate) fn transaction_source(source: &TransactionSource) -> (&'static str, Option<&str>) {
    match source {
        TransactionSource::Manual => ("manual", None),
        TransactionSource::Imported { profile } => ("imported", Some(profile.as_str())),
    }
}

/// Rebuilds a transaction source, refusing the two shapes that cannot be true.
pub(crate) fn parse_transaction_source(
    text: &str,
    profile: Option<&str>,
    table: &'static str,
) -> Result<TransactionSource, StoreError> {
    match (text, profile) {
        ("manual", None) => Ok(TransactionSource::Manual),
        ("imported", Some(profile)) => Ok(TransactionSource::Imported {
            profile: profile.to_owned(),
        }),
        ("manual", Some(_)) => Err(corrupt(
            table,
            "a manual transaction carries an import profile",
        )),
        ("imported", None) => Err(corrupt(
            table,
            "an imported transaction has no import profile",
        )),
        (other, _) => Err(corrupt(
            table,
            format!("unknown transaction source {other:?}"),
        )),
    }
}

/// A posting target as a discriminator and an optional identifier.
///
/// The opening-balance target has no identifier, which is why the target is two
/// columns rather than one: a nullable foreign key could not point at two tables
/// anyway, and a bogus identifier would be indistinguishable from a real one.
pub(crate) fn encode_target(target: PostingTarget) -> (&'static str, Option<String>) {
    match target {
        PostingTarget::Account(id) => ("account", Some(uuid(id.as_uuid()))),
        PostingTarget::Category(id) => ("category", Some(uuid(id.as_uuid()))),
        PostingTarget::Opening => ("opening", None),
    }
}

/// Rebuilds a posting target from its two columns.
pub(crate) fn decode_target(
    kind: &str,
    id: Option<&str>,
    table: &'static str,
) -> Result<PostingTarget, StoreError> {
    match (kind, id) {
        ("account", Some(id)) => Ok(PostingTarget::Account(finance_core::AccountId::from_uuid(
            parse_uuid(id, table)?,
        ))),
        ("category", Some(id)) => Ok(PostingTarget::Category(
            finance_core::CategoryId::from_uuid(parse_uuid(id, table)?),
        )),
        ("opening", None) => Ok(PostingTarget::Opening),
        ("account" | "category", None) => Err(corrupt(
            table,
            format!("a {kind} posting has no target identifier"),
        )),
        ("opening", Some(_)) => Err(corrupt(
            table,
            "an opening posting must not carry a target identifier",
        )),
        (other, _) => Err(corrupt(table, format!("unknown posting target {other:?}"))),
    }
}
