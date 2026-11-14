//! Round-trip, schema and durability tests against a real SQLite file.
//!
//! These tests never mock the database: the point is that the bytes on disk can
//! be read back, so a mock would test nothing that matters (testing strategy,
//! "Storage").

use finance_core::{
    Account, AccountId, AccountKind, Category, CategoryId, CategoryKind, Currency, Ledger, Money,
    Rule, RuleKey, RuleTarget, Transaction, TransactionId, TransactionSource, TransferPattern,
    YearMonth,
};
use finance_storage::{Store, StoreError};
use tempfile::TempDir;
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

mod support;

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

fn month(month: Month) -> YearMonth {
    YearMonth::new(2026, month)
}

/// One dataset that exercises every table: three accounts (one archived), a
/// two-level category tree, all four transaction kinds, and a budget revision.
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
        .add_category(Category::new(
            category(3),
            "Transport",
            CategoryKind::Expense,
        ))
        .expect("the category is valid");
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
    .with_note("weekly shop")
    .with_source(TransactionSource::Imported {
        profile: "bank-statement-debit-credit".to_owned(),
    });
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
    .expect("the amount is positive")
    .with_payee("Sample Employer");
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
        category(3),
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
        .set_budget(category(2), pkr(2_500_000), month(Month::October))
        .expect("a revision is allowed");

    ledger
        .add_rule(Rule::new(
            RuleKey::merchant("Sample Cafe").expect("the name is not blank"),
            RuleTarget::Category(category(2)),
        ))
        .expect("the category exists");
    ledger
        .add_rule(Rule::new(
            RuleKey::transfer(TransferPattern::peer_transfer(Some("Sample Holder"))),
            RuleTarget::Account(account(2)),
        ))
        .expect("the account exists");

    ledger
}

fn dataset(directory: &TempDir) -> std::path::PathBuf {
    directory.path().join("dataset.db")
}

#[test]
fn an_empty_dataset_loads_an_empty_ledger() {
    let store = Store::open_in_memory(Currency::PKR).expect("the dataset opens");

    assert_eq!(
        store.load().expect("the dataset loads"),
        Ledger::new(Currency::PKR)
    );
    assert_eq!(store.schema_version().expect("the version reads"), 3);
    assert_eq!(store.currency(), Currency::PKR);
}

#[test]
fn a_ledger_survives_a_round_trip() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    let ledger = fixture();

    {
        let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
        store.save(&ledger).expect("the ledger is valid");
    }

    let store = Store::open(&path, Currency::PKR).expect("the dataset reopens");
    let loaded = store.load().expect("the dataset loads");
    assert_eq!(loaded, ledger);
    assert_eq!(store.schema_version().expect("the version reads"), 3);
    assert_eq!(
        loaded
            .transaction(transaction(2))
            .expect("the imported expense is loaded")
            .source(),
        &TransactionSource::Imported {
            profile: "bank-statement-debit-credit".to_owned()
        },
        "provenance survives the round trip (FR-036)"
    );
    assert_eq!(
        loaded
            .transaction(transaction(3))
            .expect("the typed income is loaded")
            .source(),
        &TransactionSource::Manual,
        "a row nobody imported stays manual"
    );
}

#[test]
fn a_version_one_dataset_is_migrated_to_provenance() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    support::write_version_one_dataset(&path);

    let store = Store::open(&path, Currency::PKR).expect("the old dataset opens");
    assert_eq!(
        store.schema_version().expect("the version reads"),
        3,
        "opening the file applied every later migration"
    );

    let loaded = store.load().expect("the migrated dataset loads");
    assert_eq!(loaded.accounts().count(), 1, "no account was lost");
    assert_eq!(loaded.transactions().count(), 1, "no transaction was lost");
    assert_eq!(loaded.rules().count(), 0, "the file taught no rules");
    assert_eq!(
        loaded
            .transaction(transaction(1))
            .expect("the old expense is loaded")
            .source(),
        &TransactionSource::Manual,
        "a row that predates provenance is manual, not imported"
    );
    assert_eq!(
        loaded
            .balance_as_of(account(1), day(Month::August, 3))
            .expect("the account exists"),
        pkr(-125_000),
        "the posting amounts are unchanged"
    );
}

#[test]
fn a_version_two_dataset_gains_the_rules_table() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);

    // A v2 dataset is a v3 one without the rules table: the columns v2 added
    // stay, only the last migration is undone.
    let mut ledger = fixture();
    let keys: Vec<RuleKey> = ledger.rules().map(|rule| rule.key().clone()).collect();
    for key in keys {
        ledger.remove_rule(&key);
    }
    {
        let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
        store.save(&ledger).expect("the ledger is valid");
    }
    {
        let raw = rusqlite::Connection::open(&path).expect("the file is a database");
        raw.execute_batch(
            "DROP TABLE rules;
             DELETE FROM schema_migrations WHERE version = 3;",
        )
        .expect("the v2 shape is written");
    }

    let mut store = Store::open(&path, Currency::PKR).expect("the v2 dataset opens");
    assert_eq!(
        store.schema_version().expect("the version reads"),
        3,
        "opening the file applied the rules migration"
    );
    assert_eq!(
        store.load().expect("the migrated dataset loads"),
        ledger,
        "every other table came through untouched"
    );

    let key = RuleKey::merchant("Sample Cafe").expect("the name is not blank");
    let mut loaded = store.load().expect("the dataset loads");
    loaded
        .add_rule(Rule::new(key.clone(), RuleTarget::Category(category(2))))
        .expect("the category exists");
    store
        .save(&loaded)
        .expect("the migrated table accepts a rule");

    assert_eq!(
        Store::open(&path, Currency::PKR)
            .expect("the dataset reopens")
            .load()
            .expect("the dataset loads")
            .rule(&key)
            .expect("the rule was written")
            .target(),
        RuleTarget::Category(category(2))
    );
}

#[test]
fn saving_twice_changes_nothing() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    let ledger = fixture();

    let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
    store.save(&ledger).expect("the ledger is valid");
    store.save(&ledger).expect("a second save is valid");

    assert_eq!(store.load().expect("the dataset loads"), ledger);
}

#[test]
fn saving_replaces_the_live_dataset() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);

    let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
    store.save(&fixture()).expect("the ledger is valid");

    let mut smaller = Ledger::new(Currency::PKR);
    smaller
        .add_account(Account::new(
            account(9),
            "Other",
            AccountKind::Cash,
            Currency::PKR,
        ))
        .expect("the account is unique");
    store.save(&smaller).expect("the ledger is valid");

    assert_eq!(
        store.load().expect("the dataset loads"),
        smaller,
        "the store is a projection of the ledger, not an accumulation"
    );
}

#[test]
fn a_deleted_transaction_is_not_loaded() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
    store.save(&fixture()).expect("the ledger is valid");

    store
        .soft_delete_transaction(transaction(2), deleted_at())
        .expect("the transaction exists");

    let loaded = store.load().expect("the dataset loads");
    assert_eq!(loaded.transactions().count(), 4);
    assert!(loaded.transaction(transaction(2)).is_none());
    assert_eq!(
        store
            .deleted_transactions()
            .expect("the deleted rows are readable"),
        vec![transaction(2)],
        "the row is kept for undo"
    );
}

#[test]
fn a_deleted_transaction_survives_a_save() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
    store.save(&fixture()).expect("the ledger is valid");
    store
        .soft_delete_transaction(transaction(2), deleted_at())
        .expect("the transaction exists");

    let live = store.load().expect("the dataset loads");
    store.save(&live).expect("the ledger is valid");

    assert_eq!(
        store
            .deleted_transactions()
            .expect("the deleted rows are readable"),
        vec![transaction(2)]
    );
    assert!(
        store
            .load()
            .expect("the dataset loads")
            .transaction(transaction(2))
            .is_none(),
        "a save must not resurrect a deleted transaction"
    );
}

#[test]
fn restoring_brings_a_transaction_back() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    let ledger = fixture();
    let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
    store.save(&ledger).expect("the ledger is valid");

    store
        .soft_delete_transaction(transaction(2), deleted_at())
        .expect("the transaction exists");
    store
        .restore_transaction(transaction(2))
        .expect("the transaction exists");

    assert_eq!(store.load().expect("the dataset loads"), ledger);
    assert!(
        store
            .deleted_transactions()
            .expect("the deleted rows are readable")
            .is_empty()
    );
}

#[test]
fn deleting_an_unknown_transaction_fails() {
    let mut store = Store::open_in_memory(Currency::PKR).expect("the dataset opens");

    let error = store
        .soft_delete_transaction(transaction(404), deleted_at())
        .expect_err("the transaction is unknown");
    assert!(matches!(
        error,
        StoreError::UnknownTransaction(id) if id == transaction(404)
    ));

    let error = store
        .restore_transaction(transaction(404))
        .expect_err("the transaction is unknown");
    assert!(matches!(
        error,
        StoreError::UnknownTransaction(id) if id == transaction(404)
    ));
}

#[test]
fn a_dataset_in_another_currency_is_refused() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);

    Store::open(&path, Currency::PKR).expect("the dataset opens");

    let error = Store::open(&path, Currency::USD).expect_err("USD is not the dataset's currency");
    assert!(matches!(
        error,
        StoreError::CurrencyMismatch {
            stored,
            expected
        } if stored == Currency::PKR && expected == Currency::USD
    ));
}

#[test]
fn a_ledger_in_another_currency_cannot_be_saved() {
    let mut store = Store::open_in_memory(Currency::PKR).expect("the dataset opens");

    let error = store
        .save(&Ledger::new(Currency::USD))
        .expect_err("the ledger is in USD");
    assert!(matches!(
        error,
        StoreError::LedgerCurrency { ledger, store }
            if ledger == Currency::USD && store == Currency::PKR
    ));
}

#[test]
fn a_newer_schema_is_refused() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    Store::open(&path, Currency::PKR).expect("the dataset opens");

    let raw = rusqlite::Connection::open(&path).expect("the file is a database");
    raw.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (99, '2026-11-12T00:00:00Z')",
        [],
    )
    .expect("the version table exists");
    drop(raw);

    let error = Store::open(&path, Currency::PKR).expect_err("version 99 is not supported");
    assert!(matches!(
        error,
        StoreError::UnsupportedSchema {
            found: 99,
            supported: 3
        }
    ));
}

#[test]
fn a_failed_save_leaves_the_previous_dataset() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    let ledger = fixture();
    let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
    store.save(&ledger).expect("the ledger is valid");

    // Another connection takes the write lock, so the save cannot commit.
    let blocker = rusqlite::Connection::open(&path).expect("the file is a database");
    blocker
        .execute_batch("BEGIN IMMEDIATE")
        .expect("the lock is available");

    let mut smaller = Ledger::new(Currency::PKR);
    smaller
        .add_account(Account::new(
            account(9),
            "Other",
            AccountKind::Cash,
            Currency::PKR,
        ))
        .expect("the account is unique");
    let error = store.save(&smaller).expect_err("the write lock is held");
    assert!(
        matches!(error, StoreError::Database(_)),
        "a busy dataset is a database error, got {error:?}"
    );

    blocker
        .execute_batch("ROLLBACK")
        .expect("the transaction is open");

    assert_eq!(
        store.load().expect("the dataset loads"),
        ledger,
        "the half-written dataset was rolled back"
    );
}

#[test]
fn amounts_keep_their_minor_units_and_exponent() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);

    let mut ledger = Ledger::new(Currency::KWD);
    ledger
        .add_account(Account::new(
            account(1),
            "Dinar",
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
                Money::from_minor(1_234, Currency::KWD),
            )
            .expect("the amount is positive"),
        )
        .expect("the category is an expense category");

    {
        let mut store = Store::open(&path, Currency::KWD).expect("the dataset opens");
        store.save(&ledger).expect("the ledger is valid");
    }

    let store = Store::open(&path, Currency::KWD).expect("the dataset reopens");
    let loaded = store.load().expect("the dataset loads");

    assert_eq!(loaded, ledger);
    assert_eq!(loaded.currency().exponent(), 3, "three decimal places");
}

/// A fixed deletion time, so the stored text is predictable.
fn deleted_at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_760_000_000).expect("a valid timestamp")
}
