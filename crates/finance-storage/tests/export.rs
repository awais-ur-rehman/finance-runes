//! CSV export tests (FR-083).
//!
//! An export is the copy the user keeps, so these tests read the files back the
//! way a person or a spreadsheet would: plain CSV, decimal money beside its
//! currency, and a manifest that says what the files mean. Nothing here opens the
//! dataset to check the export, because the point is that it does not have to.

use std::collections::BTreeMap;
use std::path::Path;

use finance_core::{
    Account, AccountId, AccountKind, Category, CategoryId, CategoryKind, Currency, Ledger, Money,
    Rule, RuleKey, RuleTarget, Transaction, TransactionId, TransferPattern, YearMonth,
};
use finance_storage::{ExportFile, FORMAT_VERSION, MANIFEST, Store, StoreError, directory_name};
use tempfile::TempDir;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};
use uuid::Uuid;

fn account(value: u128) -> AccountId {
    AccountId::from_uuid(Uuid::from_u128(value))
}

fn category(value: u128) -> CategoryId {
    CategoryId::from_uuid(Uuid::from_u128(value))
}

fn transaction(value: u128) -> TransactionId {
    TransactionId::from_uuid(Uuid::from_u128(value))
}

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

fn day(month: Month, number: u8) -> Date {
    Date::from_calendar_date(2026, month, number).expect("a valid date")
}

fn month(value: Month) -> YearMonth {
    YearMonth::new(2026, value)
}

/// A fixed moment, so the directory name and the manifest are predictable.
fn moment() -> OffsetDateTime {
    let date = Date::from_calendar_date(2026, Month::November, 13).expect("a valid date");
    let time = Time::from_hms(12, 0, 0).expect("a valid time");
    PrimitiveDateTime::new(date, time).assume_utc()
}

/// One dataset that exercises every table.
fn fixture() -> Ledger {
    let mut ledger = Ledger::new(Currency::PKR);

    ledger
        .add_account(Account::new(
            account(1),
            "Pass-through",
            AccountKind::Bank,
            Currency::PKR,
        ))
        .expect("the account is unique");
    ledger
        .add_account(Account::new(
            account(2),
            "Cash",
            AccountKind::Cash,
            Currency::PKR,
        ))
        .expect("the account is unique");
    let mut savings = Account::new(account(3), "Savings", AccountKind::Savings, Currency::PKR);
    savings.archive();
    ledger.add_account(savings).expect("the account is unique");

    ledger
        .add_category(Category::new(category(1), "Food", CategoryKind::Expense))
        .expect("the category is valid");
    ledger
        .add_category(
            Category::new(category(2), "Groceries", CategoryKind::Expense).with_parent(category(1)),
        )
        .expect("the parent exists");
    ledger
        .add_category(Category::new(category(4), "Salary", CategoryKind::Income))
        .expect("the category is valid");

    let opening = Transaction::opening(
        transaction(1),
        day(Month::August, 1),
        account(1),
        pkr(10_000_000),
    )
    .expect("the amount is positive");
    ledger.record(opening).expect("the account exists");

    let expense = Transaction::expense(
        transaction(2),
        day(Month::August, 3),
        account(1),
        category(2),
        pkr(125_000),
    )
    .expect("the amount is positive")
    .with_payee("Sample Mart")
    .with_note("weekly shop");
    ledger
        .record(expense)
        .expect("the category is an expense category");

    let income = Transaction::income(
        transaction(3),
        day(Month::August, 5),
        account(1),
        category(4),
        pkr(5_000_000),
    )
    .expect("the amount is positive");
    ledger
        .record(income)
        .expect("the category is an income category");

    let transfer = Transaction::transfer(
        transaction(4),
        day(Month::August, 6),
        account(1),
        account(2),
        pkr(500_000),
    )
    .expect("the amount is positive");
    ledger.record(transfer).expect("both accounts exist");

    let later = Transaction::expense(
        transaction(5),
        day(Month::September, 2),
        account(2),
        category(2),
        pkr(30_000),
    )
    .expect("the amount is positive");
    ledger
        .record(later)
        .expect("the category is an expense category");

    ledger
        .set_budget(category(2), pkr(2_000_000), month(Month::August))
        .expect("the category is an expense category");
    ledger
        .add_rule(Rule::new(
            RuleKey::merchant("Sample Cafe").expect("the name is not blank"),
            RuleTarget::Category(category(2)),
        ))
        .expect("the category exists");
    ledger
        .add_rule(Rule::new(
            RuleKey::transfer(TransferPattern::CashWithdrawal),
            RuleTarget::Account(account(2)),
        ))
        .expect("the account exists");

    ledger
}

/// A saved dataset in `directory`, with the September expense soft-deleted.
fn dataset(directory: &Path) -> Store {
    let mut store =
        Store::open(directory.join("dataset.db"), Currency::PKR).expect("the dataset opens");
    store.save(&fixture()).expect("the ledger is valid");
    store
        .soft_delete_transaction(transaction(5), moment())
        .expect("the transaction exists");
    store
}

/// The header and data rows of a CSV file, read the way a spreadsheet would.
fn read_csv(path: &Path) -> (Vec<String>, Vec<Vec<String>>) {
    let mut reader = csv::Reader::from_path(path).expect("the file is readable");
    let header = reader
        .headers()
        .expect("the header reads")
        .iter()
        .map(str::to_owned)
        .collect();
    let rows = reader
        .records()
        .map(|record| {
            record
                .expect("the row reads")
                .iter()
                .map(str::to_owned)
                .collect()
        })
        .collect();
    (header, rows)
}

/// The row whose first column is `id`.
fn row_with_id<'a>(rows: &'a [Vec<String>], id: &str) -> &'a [String] {
    rows.iter()
        .find(|row| row[0] == id)
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("no row with id {id}"))
}

#[test]
fn an_export_is_one_directory_of_documented_files() {
    let directory = TempDir::new().expect("a temp directory");
    let store = dataset(directory.path());

    assert_eq!(directory_name(moment()), "finance-export-20261113T120000Z");
    let export = store
        .export_into(directory.path(), moment())
        .expect("the export is written");

    assert_eq!(
        export.path(),
        directory.path().join("finance-export-20261113T120000Z")
    );
    assert_eq!(export.created_at(), moment());

    for file in ExportFile::ALL {
        let path = export.path().join(file.name());
        assert!(path.is_file(), "{} is missing", file.name());
        let (header, rows) = read_csv(&path);
        assert!(!header.is_empty(), "{} has no header", file.name());
        assert_eq!(
            rows.len(),
            export.rows(file),
            "{} rows disagree with the report",
            file.name()
        );
    }

    assert!(
        !directory
            .path()
            .join("finance-export-20261113T120000Z.partial")
            .exists(),
        "the staging directory is renamed, not left behind"
    );
}

#[test]
fn the_manifest_names_the_format_the_time_and_the_row_counts() {
    let directory = TempDir::new().expect("a temp directory");
    let store = dataset(directory.path());
    let export = store
        .export_into(directory.path(), moment())
        .expect("the export is written");

    let (header, rows) = read_csv(&export.path().join(MANIFEST));
    assert_eq!(header, ["key", "value"]);
    let manifest: BTreeMap<String, String> = rows
        .into_iter()
        .map(|row| (row[0].clone(), row[1].clone()))
        .collect();

    assert_eq!(manifest["format_version"], FORMAT_VERSION.to_string());
    assert_eq!(manifest["schema_version"], "3");
    assert_eq!(manifest["created_at"], "2026-11-13T12:00:00Z");
    assert_eq!(manifest["currency_code"], "PKR");
    assert_eq!(manifest["currency_exponent"], "2");
    assert_eq!(manifest["transactions.csv.rows"], "5");
    assert_eq!(manifest["accounts.csv.rows"], "3");
    assert_eq!(manifest["rules.csv.rows"], "2");
}

#[test]
fn a_soft_deleted_row_is_exported_with_its_deletion_time() {
    let directory = TempDir::new().expect("a temp directory");
    let store = dataset(directory.path());
    let export = store
        .export_into(directory.path(), moment())
        .expect("the export is written");

    let (header, rows) = read_csv(&export.path().join(ExportFile::Transactions.name()));
    let deleted = row_with_id(&rows, &transaction(5).as_uuid().to_string());

    assert_eq!(
        deleted[header
            .iter()
            .position(|name| name == "deleted_at")
            .expect("the column exists")],
        "2026-11-13T12:00:00Z",
        "the export is the only place a deletion is purged, so it must be visible"
    );

    let (header, rows) = read_csv(&export.path().join(ExportFile::Postings.name()));
    let id = transaction(5).as_uuid().to_string();
    let position = header
        .iter()
        .position(|name| name == "transaction_id")
        .expect("the column exists");
    assert_eq!(
        rows.iter().filter(|row| row[position] == id).count(),
        2,
        "a deleted transaction keeps the postings that made it balance"
    );
}

#[test]
fn amounts_are_plain_decimal_text_beside_their_currency() {
    let directory = TempDir::new().expect("a temp directory");
    let store = dataset(directory.path());
    let export = store
        .export_into(directory.path(), moment())
        .expect("the export is written");

    let (header, rows) = read_csv(&export.path().join(ExportFile::Postings.name()));
    let amount = header
        .iter()
        .position(|name| name == "amount")
        .expect("the column exists");
    let target = header
        .iter()
        .position(|name| name == "target_kind")
        .expect("the column exists");
    let currency = header
        .iter()
        .position(|name| name == "currency_code")
        .expect("the column exists");

    let id = transaction(2).as_uuid().to_string();
    let outflow = rows
        .iter()
        .find(|row| row[0] == id && row[target] == "account")
        .expect("the expense has an account posting");

    assert_eq!(outflow[amount], "-1250.00");
    assert_eq!(outflow[currency], "PKR");

    let (_, rows) = read_csv(&export.path().join(ExportFile::Budgets.name()));
    assert_eq!(
        rows[0][2], "20000.00",
        "a budget is decimal text too, not minor units"
    );
}

#[test]
fn a_three_decimal_currency_keeps_its_third_decimal() {
    let directory = TempDir::new().expect("a temp directory");
    let mut store =
        Store::open(directory.path().join("dataset.db"), Currency::KWD).expect("the dataset opens");
    let mut ledger = Ledger::new(Currency::KWD);
    ledger
        .add_account(Account::new(
            account(1),
            "Bank",
            AccountKind::Bank,
            Currency::KWD,
        ))
        .expect("the account is unique");
    ledger
        .add_category(Category::new(category(1), "Food", CategoryKind::Expense))
        .expect("the category is valid");
    ledger
        .record(
            Transaction::expense(
                transaction(1),
                day(Month::August, 3),
                account(1),
                category(1),
                Money::from_minor(1234, Currency::KWD),
            )
            .expect("the amount is positive"),
        )
        .expect("the category is an expense category");
    store.save(&ledger).expect("the ledger is valid");

    let export = store
        .export_into(directory.path(), moment())
        .expect("the export is written");
    let (header, rows) = read_csv(&export.path().join(ExportFile::Postings.name()));
    let amount = header
        .iter()
        .position(|name| name == "amount")
        .expect("the column exists");

    assert_eq!(rows[0][amount], "-1.234");
}

#[test]
fn a_second_export_at_the_same_moment_is_refused() {
    let directory = TempDir::new().expect("a temp directory");
    let store = dataset(directory.path());
    let first = store
        .export_into(directory.path(), moment())
        .expect("the export is written");

    let error = store
        .export_into(directory.path(), moment())
        .expect_err("the directory is taken");
    assert!(matches!(
        error,
        StoreError::ExportExists { path } if path == first.path()
    ));

    assert_eq!(
        std::fs::read_dir(first.path())
            .expect("the directory is readable")
            .count(),
        ExportFile::ALL.len(),
        "the refused export did not touch the first"
    );
}

#[test]
fn a_leftover_staging_directory_is_replaced() {
    let directory = TempDir::new().expect("a temp directory");
    let store = dataset(directory.path());
    let staging = directory
        .path()
        .join("finance-export-20261113T120000Z.partial");
    std::fs::create_dir_all(staging.join("stale")).expect("the staging directory is created");
    std::fs::write(staging.join("stale").join("left.txt"), "half an export")
        .expect("the leftover is written");

    let export = store
        .export_into(directory.path(), moment())
        .expect("the export is written");

    assert!(export.path().is_dir());
    assert!(
        !staging.exists(),
        "a crashed export is not left to look finished"
    );
}

#[test]
fn an_empty_dataset_still_writes_every_file() {
    let directory = TempDir::new().expect("a temp directory");
    let store = Store::open_in_memory(Currency::PKR).expect("the dataset opens");

    let export = store
        .export_into(directory.path(), moment())
        .expect("the export is written");

    for file in ExportFile::ALL {
        let (header, rows) = read_csv(&export.path().join(file.name()));
        assert!(!header.is_empty(), "{} has no header", file.name());
        assert_eq!(export.rows(file), rows.len());
    }
    assert_eq!(
        export.rows(ExportFile::Transactions),
        0,
        "an empty dataset exports its shape, not its absence"
    );
}
