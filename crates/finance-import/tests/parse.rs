//! End-to-end parsing of the confirmed bank shape.
//!
//! Every row here is synthetic: invented names, amounts and reference numbers.
//! The real statement is not in this repository and never will be. See
//! `docs/requirements/csv-import-profile.md`.

use finance_core::{Currency, Money, TransferPattern};
use finance_import::{
    Direction, DuplicateKey, ImportError, Profile, ReferenceKind, RowProblem, Statement, parse,
};
use time::{Date, Month};

const FIXTURE: &str = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,1250.00
12-Aug-2026,RA AST P2P FT FROM SAMPLE HOLDER HBL ACCT: PK00TEST0000. **103 MSGID: MSG000000000000000001,,0,5000.00,6250.00
12-Aug-2026,POS SALE - VISA AT SAMPLE CAFE KARACHI PK,,840.00,0,5410.00
13-Aug-2026,POS SALE - VISA AT SAMPLE MART LAHORE PK,,1200.50,0,4209.50
13-Aug-2026,ATM CASH WITHDRAWAL SAMPLE BRANCH,,2000.00,0,2209.50
";

fn profile() -> Profile {
    Profile::bank_statement_debit_credit()
}

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

fn date(month: Month, day: u8) -> Date {
    Date::from_calendar_date(2026, month, day).expect("the test date is valid")
}

fn parse_fixture() -> Statement {
    parse(FIXTURE.as_bytes(), &profile(), Currency::PKR).expect("the fixture parses")
}

#[test]
fn the_confirmed_statement_parses_to_the_documented_totals() {
    let statement = parse_fixture();

    assert_eq!(statement.header_line(), 1);
    assert_eq!(statement.data_rows(), 5);
    assert_eq!(statement.rows().len(), 4);
    assert!(statement.issues().is_empty());
    assert_eq!(
        statement
            .opening_balance()
            .expect("the marker row")
            .amount(),
        pkr(125_000)
    );
    assert_eq!(statement.total_in(), pkr(500_000));
    assert_eq!(statement.total_out(), pkr(404_050));
    assert!(statement.balance_breaks().is_empty());
    assert!(statement.duplicates().is_empty());
    assert_eq!(
        statement.date_range(),
        Some((date(Month::August, 12), date(Month::August, 13)))
    );
}

#[test]
fn the_statement_records_the_profile_that_read_it() {
    let statement = parse_fixture();

    assert_eq!(
        statement.profile(),
        "bank-statement-debit-credit",
        "an imported row can say which layout produced it (FR-036)"
    );
}

#[test]
fn the_preview_reports_the_final_balance_and_the_suspected_transfers() {
    let preview = parse_fixture().preview();

    assert_eq!(preview.final_balance(), Some(pkr(220_950)));
    assert_eq!(
        preview.suspected_transfers(),
        2,
        "the peer transfer and the cash withdrawal are the owner's own money"
    );
    assert!(preview.is_clean());
}

#[test]
fn a_peer_transfer_is_a_hint_and_carries_a_reference() {
    let statement = parse_fixture();
    let transfer = &statement.rows()[0];

    assert!(transfer.flow().is_incoming());
    assert_eq!(transfer.flow().amount(), pkr(500_000));
    assert_eq!(
        transfer.transfer_hint(),
        Some(TransferPattern::PeerTransfer {
            counterparty: Some("SAMPLE HOLDER".to_owned()),
        })
    );
    assert_eq!(transfer.merchant(), None);
    assert_eq!(
        transfer.duplicate_key(),
        DuplicateKey::Reference {
            kind: ReferenceKind::MessageId,
            value: "MSG000000000000000001".to_owned(),
        }
    );
}

#[test]
fn a_card_row_offers_its_merchant_and_keeps_the_description() {
    let statement = parse_fixture();
    let purchase = &statement.rows()[1];

    assert_eq!(
        purchase.description(),
        "POS SALE - VISA AT SAMPLE CAFE KARACHI PK",
        "the raw text is what the register shows"
    );
    assert_eq!(purchase.merchant(), Some("SAMPLE CAFE".to_owned()));
    assert_eq!(purchase.flow().signed(), pkr(-84_000));
    assert_eq!(purchase.balance(), Some(pkr(541_000)));
}

#[test]
fn the_header_is_found_below_the_banks_title_lines() {
    let text = format!("ACCOUNT STATEMENT\nSAMPLE BANK\n{FIXTURE}");

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.header_line(), 3);
    assert_eq!(statement.data_rows(), 5);
    assert_eq!(statement.rows().len(), 4);
}

#[test]
fn a_byte_order_mark_and_windows_line_endings_are_accepted() {
    let text = format!("\u{feff}{}", FIXTURE.replace('\n', "\r\n"));

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.rows().len(), 4);
    assert_eq!(statement.total_out(), pkr(404_050));
}

#[test]
fn utf16_is_decoded() {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in FIXTURE.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }

    let statement = parse(&bytes, &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.rows().len(), 4);
    assert_eq!(
        statement.rows()[1].merchant(),
        Some("SAMPLE CAFE".to_owned())
    );
}

#[test]
fn a_quoted_comma_stays_in_the_description() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,1000.00
12-Aug-2026,\"POS SALE - VISA AT SAMPLE CAFE, KARACHI PK\",,840.00,0,160.00
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.rows().len(), 1);
    assert_eq!(
        statement.rows()[0].description(),
        "POS SALE - VISA AT SAMPLE CAFE, KARACHI PK"
    );
    assert_eq!(
        statement.rows()[0].merchant(),
        Some("SAMPLE CAFE".to_owned()),
        "the separating comma is not part of the merchant"
    );
}

#[test]
fn a_bad_date_is_reported_with_its_line_and_the_rest_is_parsed() {
    let text = FIXTURE.replace("12-Aug-2026,POS SALE", "32-Aug-2026,POS SALE");

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.rows().len(), 3);
    assert_eq!(statement.issues().len(), 1);
    assert_eq!(statement.issues()[0].line(), 4);
    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::Date { .. }
    ));
}

#[test]
fn every_data_row_is_in_exactly_one_bucket() {
    let text = FIXTURE.replace("12-Aug-2026,POS SALE", "32-Aug-2026,POS SALE");
    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    let accounted = statement.rows().len()
        + statement.issues().len()
        + usize::from(statement.opening_balance().is_some());

    assert_eq!(accounted, statement.data_rows());
}

#[test]
fn a_balance_break_names_the_first_offending_line_and_both_balances() {
    let text = FIXTURE.replace("840.00,0,5410.00", "900.00,0,5410.00");

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");
    let balance_break = statement
        .first_balance_break()
        .expect("the altered amount breaks the running balance");

    assert_eq!(balance_break.line(), 4);
    assert_eq!(balance_break.expected(), pkr(535_000));
    assert_eq!(balance_break.found(), pkr(541_000));
    assert_eq!(
        statement.balance_breaks().len(),
        1,
        "the check resynchronises instead of cascading"
    );
}

#[test]
fn an_unreadable_balance_is_an_issue_and_stops_the_check() {
    let text = FIXTURE.replace("840.00,0,5410.00", "840.00,0,n/a");

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.issues().len(), 1);
    assert_eq!(statement.issues()[0].line(), 4);
    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::Balance { .. }
    ));
    assert!(
        statement.balance_breaks().is_empty(),
        "nothing after an unreadable balance can be verified, and guessing would be a false alarm"
    );
}

#[test]
fn a_repeated_row_is_reported_as_a_duplicate() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,POS SALE - VISA AT SAMPLE CAFE KARACHI PK,,840.00,0,9160.00
12-Aug-2026,POS SALE - VISA AT SAMPLE CAFE KARACHI PK,,840.00,0,8320.00
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.rows().len(), 2);
    assert!(statement.balance_breaks().is_empty());
    assert_eq!(statement.duplicates().len(), 1);
    assert_eq!(statement.duplicates()[0].line(), 3);
    assert_eq!(statement.duplicates()[0].first_line(), 2);
    assert!(!statement.preview().is_clean());
}

#[test]
fn both_directions_filled_is_an_issue() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,CONFUSING ROW,,840.00,100.00,1000.00
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert!(statement.rows().is_empty());
    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::BothDirections { .. }
    ));
}

#[test]
fn a_negative_amount_in_the_debit_column_is_an_issue() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,NEGATIVE ROW,,-840.00,0,1000.00
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::Negative { column, .. } if column == "DEBIT"
    ));
}

#[test]
fn a_row_with_the_wrong_number_of_columns_is_an_issue() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,SHORT ROW,,840.00,0
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::WrongColumnCount {
            expected: 6,
            found: 5
        }
    ));
}

#[test]
fn an_empty_description_is_an_issue() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,,,840.00,0,160.00
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::NoDescription
    ));
}

#[test]
fn a_zero_row_that_is_not_an_opening_balance_is_an_issue() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,NOTHING MOVED,,0,0,1000.00
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::NoAmount
    ));
}

#[test]
fn a_second_opening_balance_is_an_issue() {
    let text = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,1000.00
12-Aug-2026,** Opening Balance,,0,0,1000.00
";

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert!(statement.rows().is_empty());
    assert_eq!(statement.issues().len(), 1);
    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::RepeatedOpeningBalance
    ));
}

#[test]
fn an_opening_balance_without_a_balance_column_is_an_issue() {
    let profile = Profile::new(
        "no-balance",
        "DATE",
        "PARTICULARS",
        Direction::DebitCredit {
            debit: "DEBIT".to_owned(),
            credit: "CREDIT".to_owned(),
        },
    );
    let text = "\
DATE,PARTICULARS,DEBIT,CREDIT
12-Aug-2026,** Opening Balance,0,0
";

    let statement = parse(text.as_bytes(), &profile, Currency::PKR).expect("the file parses");

    assert!(matches!(
        statement.issues()[0].problem(),
        RowProblem::OpeningBalanceWithoutBalance
    ));
}

#[test]
fn a_signed_amount_column_reads_direction_from_the_sign() {
    let profile = Profile::new(
        "one-column",
        "DATE",
        "PARTICULARS",
        Direction::Signed {
            amount: "AMOUNT".to_owned(),
        },
    )
    .with_balance("BALANCE");
    let text = "\
DATE,PARTICULARS,AMOUNT,BALANCE
12-Aug-2026,** Opening Balance,0,1000.00
12-Aug-2026,SALARY,5000.00,6000.00
12-Aug-2026,GROCERIES,-1500.00,4500.00
";

    let statement = parse(text.as_bytes(), &profile, Currency::PKR).expect("the file parses");

    assert_eq!(statement.rows().len(), 2);
    assert!(statement.rows()[0].flow().is_incoming());
    assert_eq!(statement.rows()[1].flow().signed(), pkr(-150_000));
    assert_eq!(statement.total_in(), pkr(500_000));
    assert_eq!(statement.total_out(), pkr(150_000));
    assert!(statement.balance_breaks().is_empty());
}

#[test]
fn a_blank_line_is_not_a_row() {
    let text = FIXTURE.replace("1250.00\n", "1250.00\n\n");

    let statement = parse(text.as_bytes(), &profile(), Currency::PKR).expect("the file parses");

    assert_eq!(statement.data_rows(), 5);
    assert_eq!(statement.rows().len(), 4);
}

#[test]
fn an_empty_file_is_an_error() {
    let error = parse(b"  \n", &profile(), Currency::PKR).expect_err("there is nothing to parse");

    assert!(matches!(error, ImportError::Empty));
}

#[test]
fn a_file_without_a_header_is_an_error() {
    let error =
        parse(b"just prose\nand more\n", &profile(), Currency::PKR).expect_err("no header row");

    assert!(matches!(error, ImportError::MissingHeader { .. }));
}

#[test]
fn a_header_missing_a_profile_column_is_an_error() {
    let text = "DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT\n12-Aug-2026,A ROW,,0,0\n";

    let error = parse(text.as_bytes(), &profile(), Currency::PKR).expect_err("no balance column");

    assert!(matches!(error, ImportError::MissingColumn { field } if field == "BALANCE"));
}

#[test]
fn a_header_naming_a_column_twice_is_an_error() {
    let text = "DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE,DEBIT\n";

    let error = parse(text.as_bytes(), &profile(), Currency::PKR).expect_err("ambiguous header");

    assert!(matches!(error, ImportError::DuplicateColumn { field } if field == "DEBIT"));
}
