//! Crash injection: a process killed in the middle of a write (NFR-003).
//!
//! The rollback path is already covered by a competing-writer test; this one
//! kills a real process, which is the failure the durability settings exist for.
//! A test binary can spawn itself, so the writer is a second copy of this file
//! running one test, and the parent kills it wherever it happens to be.
//!
//! What must be true afterwards is not "the write survived" but "the dataset is
//! readable, passes its integrity check, and holds a consistent prefix of the
//! writes" — a crash may lose the transaction in flight, and must never lose
//! more than that.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use finance_core::{
    Account, AccountId, AccountKind, Category, CategoryId, CategoryKind, Currency, Ledger, Money,
    Transaction, TransactionId,
};
use finance_storage::Store;
use tempfile::TempDir;
use time::{Date, Month};
use uuid::Uuid;

/// Set on the child process: where it writes.
const DATASET: &str = "FINANCE_CRASH_DATASET";

/// Set on the child process: the file it touches after its first committed save,
/// so the parent knows the writer is running rather than failing to start.
const READY: &str = "FINANCE_CRASH_READY";

fn account() -> AccountId {
    AccountId::from_uuid(Uuid::from_u128(1))
}

fn category() -> CategoryId {
    CategoryId::from_uuid(Uuid::from_u128(1))
}

fn day() -> Date {
    Date::from_calendar_date(2026, Month::August, 1).expect("a valid date")
}

/// One account, one category and one expense, so the child has something to
/// append to.
fn fixture() -> Ledger {
    let mut ledger = Ledger::new(Currency::PKR);
    ledger
        .add_account(Account::new(
            account(),
            "Bank",
            AccountKind::Bank,
            Currency::PKR,
        ))
        .expect("the account is unique");
    ledger
        .add_category(Category::new(category(), "Food", CategoryKind::Expense))
        .expect("the category is valid");
    ledger
        .record(
            Transaction::expense(
                TransactionId::from_uuid(Uuid::from_u128(1)),
                day(),
                account(),
                category(),
                Money::from_minor(100, Currency::PKR),
            )
            .expect("the amount is positive"),
        )
        .expect("the category is an expense category");
    ledger
}

#[test]
fn a_dataset_survives_a_process_killed_mid_write() {
    let directory = TempDir::new().expect("a temp directory");
    let dataset = directory.path().join("dataset.db");
    let ready = directory.path().join("ready");

    let mut child = Command::new(std::env::current_exe().expect("the test binary"))
        .args(["--exact", "crash_writer", "--nocapture"])
        .env(DATASET, &dataset)
        .env(READY, &ready)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the writer starts");

    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        assert!(
            Instant::now() < deadline,
            "the writer never committed a save"
        );
        assert!(
            child.try_wait().expect("the child can be polled").is_none(),
            "the writer exited instead of writing"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    // Well into the loop, so the kill lands in the middle of a save rather than
    // between two of them.
    std::thread::sleep(Duration::from_millis(200));
    child.kill().expect("the writer is killed");
    child.wait().expect("the writer is reaped");

    let store = Store::open(&dataset, Currency::PKR).expect("the dataset reopens");
    let report = store.integrity_check().expect("the check runs");
    assert!(
        report.is_ok(),
        "a kill must not corrupt the dataset: {:?}",
        report.problems()
    );

    let ledger = store.load().expect("the dataset loads");
    assert!(
        ledger.transactions().count() >= 1,
        "the save that completed before the kill survives"
    );
}

/// The writer. Does nothing unless the parent started this process.
#[test]
fn crash_writer() {
    let Some(dataset) = std::env::var_os(DATASET) else {
        return;
    };
    let dataset = PathBuf::from(dataset);
    let ready = PathBuf::from(std::env::var_os(READY).expect("the marker path is set"));

    let mut store = Store::open(&dataset, Currency::PKR).expect("the dataset opens");
    let mut ledger = fixture();
    store.save(&ledger).expect("the first save is valid");
    std::fs::write(&ready, b"ready").expect("the marker is written");

    let mut next = 2_u128;
    loop {
        let transaction = Transaction::expense(
            TransactionId::from_uuid(Uuid::from_u128(next)),
            day(),
            account(),
            category(),
            Money::from_minor(100, Currency::PKR),
        )
        .expect("the amount is positive");
        ledger
            .record(transaction)
            .expect("the category is an expense category");
        store.save(&ledger).expect("the save is valid");
        next += 1;
    }
}
