//! Pre-migration backup tests (FR-084).
//!
//! The point of the copy is that an upgrade which turns out to be wrong is one
//! file rename away from being undone, so these tests check the copy itself: that
//! it exists, that it is a complete dataset at the old version, and that it is
//! taken only when an upgrade actually happens.

use std::path::Path;

use finance_core::{Account, AccountId, AccountKind, Currency, Ledger};
use finance_storage::Store;
use rusqlite::Connection;
use tempfile::TempDir;
use uuid::Uuid;

mod support;

fn dataset(directory: &TempDir) -> std::path::PathBuf {
    directory.path().join("dataset.db")
}

/// The schema version recorded in a file, read without opening a [`Store`].
fn version_in(path: &Path) -> i64 {
    Connection::open(path)
        .expect("the file is a database")
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("the version reads")
}

#[test]
fn an_upgrade_keeps_the_dataset_as_it_was() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    support::write_version_one_dataset(&path);

    let store = Store::open(&path, Currency::PKR).expect("the old dataset opens");
    let copy = store
        .migration_backup()
        .expect("the upgrade kept a copy")
        .to_path_buf();

    assert_eq!(copy, directory.path().join("dataset.db.pre-v1.db"));
    assert!(copy.is_file());
    assert!(
        !directory.path().join("dataset.db.pre-v1.partial").exists(),
        "the staging file is renamed, not left behind"
    );
    assert!(
        !directory.path().join("dataset.db.pre-v1.db-wal").exists(),
        "the copy is one self-contained file"
    );
    assert!(!directory.path().join("dataset.db.pre-v1.db-shm").exists());

    assert_eq!(version_in(&copy), 1, "the copy is still the old version");
    let name: String = Connection::open(&copy)
        .expect("the copy opens")
        .query_row("SELECT name FROM accounts", [], |row| row.get(0))
        .expect("the copy holds the account");
    assert_eq!(name, "Pass-through", "the copy holds the old data");

    assert_eq!(
        store.schema_version().expect("the version reads"),
        3,
        "the dataset itself was upgraded"
    );
    assert_eq!(
        store
            .load()
            .expect("the upgraded dataset loads")
            .accounts()
            .count(),
        1
    );
}

#[test]
fn a_dataset_that_needs_no_upgrade_is_not_copied() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    {
        let mut store = Store::open(&path, Currency::PKR).expect("the dataset opens");
        let mut ledger = Ledger::new(Currency::PKR);
        ledger
            .add_account(Account::new(
                AccountId::from_uuid(Uuid::from_u128(1)),
                "Bank",
                AccountKind::Bank,
                Currency::PKR,
            ))
            .expect("the account is unique");
        store.save(&ledger).expect("the ledger is valid");
    }

    let store = Store::open(&path, Currency::PKR).expect("the dataset reopens");

    assert_eq!(store.migration_backup(), None);
    assert!(
        !directory.path().join("dataset.db.pre-v3.db").exists(),
        "opening a current dataset does not copy it"
    );
}

#[test]
fn a_new_dataset_is_not_copied() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);

    let store = Store::open(&path, Currency::PKR).expect("the dataset is created");

    assert_eq!(store.migration_backup(), None);
    assert!(
        !directory.path().join("dataset.db.pre-v0.db").exists(),
        "there is nothing to copy before the first schema exists"
    );
}

#[test]
fn a_leftover_copy_is_replaced_with_a_fresh_one() {
    let directory = TempDir::new().expect("a temp directory");
    let path = dataset(&directory);
    support::write_version_one_dataset(&path);

    let stale = directory.path().join("dataset.db.pre-v1.db");
    std::fs::write(&stale, "half a copy from a crashed run").expect("the stale copy is written");

    let store = Store::open(&path, Currency::PKR).expect("the old dataset opens");

    assert_eq!(store.migration_backup(), Some(stale.as_path()));
    assert_eq!(
        version_in(&stale),
        1,
        "a stale file at the same name is replaced, not trusted"
    );
}

#[test]
fn an_in_memory_dataset_has_nothing_to_copy() {
    let store = Store::open_in_memory(Currency::PKR).expect("the dataset opens");

    assert_eq!(store.migration_backup(), None);
}
