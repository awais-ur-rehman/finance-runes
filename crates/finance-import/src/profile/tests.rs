use crate::date::DateFormat;

use super::{Direction, Profile};

#[test]
fn the_confirmed_profile_names_the_six_columns() {
    let profile = Profile::bank_statement_debit_credit();

    assert_eq!(profile.name(), "bank-statement-debit-credit");
    assert_eq!(profile.date_column(), "DATE");
    assert_eq!(profile.description_column(), "PARTICULARS");
    assert_eq!(profile.instrument_column(), Some("INST. NO."));
    assert_eq!(profile.balance_column(), Some("BALANCE"));
    assert_eq!(profile.date_format(), DateFormat::DayMonthNameYear);
}

#[test]
fn required_columns_are_the_ones_the_header_is_found_by() {
    let profile = Profile::bank_statement_debit_credit();

    assert_eq!(
        profile.required_columns(),
        vec!["DATE", "PARTICULARS", "DEBIT", "CREDIT"],
        "a header without a balance column is still the header"
    );
    assert_eq!(
        profile.columns(),
        vec![
            "DATE",
            "PARTICULARS",
            "DEBIT",
            "CREDIT",
            "INST. NO.",
            "BALANCE"
        ]
    );
}

#[test]
fn a_signed_profile_requires_one_amount_column() {
    let profile = Profile::new(
        "one-column",
        "Date",
        "Details",
        Direction::Signed {
            amount: "Amount".to_owned(),
        },
    );

    assert_eq!(
        profile.required_columns(),
        vec!["Date", "Details", "Amount"]
    );
    assert_eq!(profile.instrument_column(), None);
    assert_eq!(profile.balance_column(), None);
}

#[test]
fn the_date_format_can_be_changed() {
    let profile = Profile::bank_statement_debit_credit().with_date_format(DateFormat::IsoDate);

    assert_eq!(profile.date_format(), DateFormat::IsoDate);
}
