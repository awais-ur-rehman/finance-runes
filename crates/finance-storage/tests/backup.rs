//! Backup, retention, restore and integrity tests.
//!
//! Restore replaces the only copy of the user's history, so most of these tests
//! are about what must *not* happen: a refused file must leave the current
//! dataset untouched, and pruning must never delete a file this app did not
//! write.

use std::path::Path;

use finance_core::{
    Account, AccountId, AccountKind, Category, CategoryId, CategoryKind, Currency, Ledger, Money,
    Transaction, TransactionId,
};
use finance_storage::{KEEP_BACKUPS, Store, StoreError, file_name, is_backup_name, prune, restore};
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

/// A fixed moment, so file names are predictable.
fn moment(hour: u8) -> OffsetDateTime {
    let date = Date::from_calendar_date(2026, Month::November, 12).expect("a valid date");
    let time = Time::from_hms(hour, 0, 0).expect("a valid time");
    PrimitiveDateTime::new(date, time).assume_utc()
}

fn fixture() -> Ledger {
    let mut ledger = Ledger::new(Currency::PKR);
    ledger
        .add_account(Account::new(
            account(1),
            "Bank",
            AccountKind::Bank,
            Currency::PKR,
        ))
        .expect("the account is unique");
    ledger
        .add_category(Category::new(category(1), "Food", CategoryKind::Expense))
        .expect("the category is valid");
    ledger
        .record(
            Transaction::expense(
                transaction(1),
                Date::from_calendar_date(2026, Month::August, 3).expect("a valid date"),
                account(1),
                category(1),
                pkr(125_000),
            )
            .expect("the amount is positive"),
        )
        .expect("the category is an expense category");
    ledger
}

fn names_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .expect("the directory exists")
        .map(|entry| {
            entry
                .expect("the entry is readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn a_backup_is_one_self_contained_file() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    let ledger = fixture();

    let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
    store.save(&ledger).expect("the ledger is valid");

    let path = directory.path().join("copy.db");
    let backup = store
        .backup_to(&path, moment(14))
        .expect("the backup is written");

    assert_eq!(backup.path(), path.as_path());
    assert_eq!(backup.schema_version(), 3);
    assert_eq!(backup.created_at(), moment(14));
    assert!(
        !directory.path().join("copy.db-wal").exists(),
        "a backup must not depend on a write-ahead log"
    );
    assert!(!directory.path().join("copy.db-shm").exists());

    let copy = Store::open(&path, Currency::PKR).expect("the backup opens");
    assert_eq!(copy.load().expect("the backup loads"), ledger);
}

#[test]
fn a_backup_name_carries_its_time_and_version() {
    assert_eq!(
        file_name(moment(14), 1),
        "finance-20261112T140000Z-v1.db",
        "fixed-width UTC so the names sort by age"
    );
    assert!(is_backup_name("finance-20261112T140000Z-v1.db"));
    assert!(
        !is_backup_name("finance-20261112T140000Z-v1.db.bak"),
        "a file we did not write"
    );
    assert!(!is_backup_name("finance-manual.db"));
    assert!(!is_backup_name("notes.txt"));
    assert!(!is_backup_name("finance-2026Z-v1.db"));
}

#[test]
fn backups_are_kept_to_ten_and_the_oldest_go_first() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    let backups = directory.path().join("backups");

    let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
    store.save(&fixture()).expect("the ledger is valid");

    for hour in 0..12 {
        store
            .backup_into(&backups, moment(hour))
            .expect("the backup is written");
    }

    let names = names_in(&backups);
    assert_eq!(names.len(), KEEP_BACKUPS);
    assert_eq!(
        names.first().expect("ten backups remain"),
        "finance-20261112T020000Z-v3.db",
        "the two oldest were pruned"
    );
    assert_eq!(
        names.last().expect("ten backups remain"),
        "finance-20261112T110000Z-v3.db"
    );
}

#[test]
fn pruning_leaves_files_it_does_not_recognise() {
    let directory = TempDir::new().expect("a temp directory");
    for name in [
        "notes.txt",
        "finance-manual.db",
        "finance-20261112T000000Z-v1.db.bak",
        "finance-20261112T000000Z-v1.db",
    ] {
        std::fs::write(directory.path().join(name), b"contents").expect("the file is written");
    }

    let removed = prune(directory.path(), 0).expect("the directory is readable");

    assert_eq!(removed, 1, "only the real backup is a candidate");
    assert!(directory.path().join("notes.txt").exists());
    assert!(directory.path().join("finance-manual.db").exists());
    assert!(
        directory
            .path()
            .join("finance-20261112T000000Z-v1.db.bak")
            .exists()
    );
    assert!(
        !directory
            .path()
            .join("finance-20261112T000000Z-v1.db")
            .exists()
    );
}

#[test]
fn a_backup_clears_the_change_flag() {
    let directory = TempDir::new().expect("a temp directory");
    let mut store =
        Store::open(directory.path().join("dataset.db"), Currency::PKR).expect("the dataset opens");

    assert!(!store.needs_backup(), "nothing has changed yet");

    store.save(&fixture()).expect("the ledger is valid");
    assert!(store.needs_backup(), "a save is a change");

    store
        .backup_into(directory.path().join("backups"), moment(14))
        .expect("the backup is written");
    assert!(!store.needs_backup(), "the change is backed up");
}

#[test]
fn restoring_replaces_the_dataset() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    let original = fixture();

    let backup_path = {
        let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
        store.save(&original).expect("the ledger is valid");
        store
            .backup_into(directory.path().join("backups"), moment(14))
            .expect("the backup is written")
            .path()
            .to_path_buf()
    };

    {
        let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset reopens");
        store
            .save(&Ledger::new(Currency::PKR))
            .expect("an empty ledger is valid");
    }

    let version = restore(&backup_path, &dataset).expect("the backup is restorable");
    assert_eq!(version, 3);

    assert!(
        !directory.path().join("dataset.restore").exists(),
        "the temporary copy is renamed, not left behind"
    );

    let store = Store::open(&dataset, Currency::PKR).expect("the dataset reopens");
    assert_eq!(store.load().expect("the dataset loads"), original);
}

#[test]
fn a_newer_backup_is_refused_and_the_dataset_is_untouched() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    let current = fixture();

    {
        let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
        store.save(&current).expect("the ledger is valid");
    }

    let future = directory.path().join("future.db");
    {
        let mut store = Store::open(&future, Currency::PKR).expect("the copy opens");
        store.save(&fixture()).expect("the ledger is valid");
    }
    let raw = rusqlite::Connection::open(&future).expect("the copy is a database");
    raw.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (99, '2026-11-12T00:00:00Z')",
        [],
    )
    .expect("the version table exists");
    drop(raw);

    let error = restore(&future, &dataset).expect_err("version 99 is not supported");
    assert!(matches!(
        error,
        StoreError::UnsupportedSchema {
            found: 99,
            supported: 3
        }
    ));

    let store = Store::open(&dataset, Currency::PKR).expect("the dataset still opens");
    assert_eq!(store.load().expect("the dataset loads"), current);
}

#[test]
fn a_file_that_is_not_a_dataset_is_refused() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    let current = fixture();
    {
        let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
        store.save(&current).expect("the ledger is valid");
    }

    let garbage = directory.path().join("garbage.db");
    std::fs::write(&garbage, b"this is not a database").expect("the file is written");

    let error = restore(&garbage, &dataset).expect_err("the file is not a database");
    assert!(
        matches!(error, StoreError::Database(_)),
        "a file SQLite cannot read is a database error, got {error:?}"
    );

    let store = Store::open(&dataset, Currency::PKR).expect("the dataset still opens");
    assert_eq!(store.load().expect("the dataset loads"), current);
}

#[test]
fn a_file_without_a_schema_is_refused() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    {
        let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
        store.save(&fixture()).expect("the ledger is valid");
    }

    let empty = directory.path().join("empty.db");
    drop(rusqlite::Connection::open(&empty).expect("an empty database is created"));

    let error = restore(&empty, &dataset).expect_err("the file holds no schema");
    assert!(matches!(error, StoreError::Corrupt { table, .. } if table == "schema_migrations"));
}

#[test]
fn a_backup_never_overwrites_an_existing_file() {
    let directory = TempDir::new().expect("a temp directory");
    let mut store =
        Store::open(directory.path().join("dataset.db"), Currency::PKR).expect("the dataset opens");
    store.save(&fixture()).expect("the ledger is valid");

    let path = directory.path().join("copy.db");
    store
        .backup_to(&path, moment(14))
        .expect("the first backup is written");

    let error = store
        .backup_to(&path, moment(15))
        .expect_err("the path is taken");
    assert!(matches!(error, StoreError::BackupExists { path: taken } if taken == path));
}

#[test]
fn restoring_discards_a_stale_write_ahead_log() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    let original = fixture();

    let backup_path = {
        let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
        store.save(&original).expect("the ledger is valid");
        store
            .backup_into(directory.path().join("backups"), moment(14))
            .expect("the backup is written")
            .path()
            .to_path_buf()
    };

    // A log left over from the dataset being replaced would be applied to the
    // restored file if it survived.
    std::fs::write(directory.path().join("dataset.db-wal"), b"stale log")
        .expect("the log is written");

    restore(&backup_path, &dataset).expect("the backup is restorable");

    assert!(
        !directory.path().join("dataset.db-wal").exists(),
        "the old log must not outlive the restore"
    );
    assert_eq!(
        Store::open(&dataset, Currency::PKR)
            .expect("the dataset reopens")
            .load()
            .expect("the dataset loads"),
        original
    );
}

#[test]
fn a_healthy_dataset_passes_its_integrity_check() {
    let mut store = Store::open_in_memory(Currency::PKR).expect("the dataset opens");
    store.save(&fixture()).expect("the ledger is valid");

    let report = store.integrity_check().expect("the check runs");

    assert!(report.is_ok());
    assert!(report.problems().is_empty());
}
