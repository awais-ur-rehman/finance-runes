//! Fixtures shared by the storage integration tests.
//!
//! The historical schema is frozen here on purpose: it is what an old build
//! actually wrote, so a migration test cannot drift with the current schema and
//! end up testing the current one against itself.

use std::path::Path;

use finance_core::{AccountId, CategoryId, TransactionId};
use rusqlite::params;
use uuid::Uuid;

/// The version-one schema, exactly as the build before provenance wrote it: no
/// `source` columns, no `rules` table, and only migration 1 recorded.
const V1: &str = "\
CREATE TABLE schema_migrations (
    version    INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
) STRICT;

CREATE TABLE metadata (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

CREATE TABLE accounts (
    id                TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    kind              TEXT NOT NULL,
    currency_code     TEXT NOT NULL,
    currency_exponent INTEGER NOT NULL,
    archived          INTEGER NOT NULL CHECK (archived IN (0, 1))
) STRICT;

CREATE TABLE categories (
    id     TEXT PRIMARY KEY,
    name   TEXT NOT NULL,
    kind   TEXT NOT NULL,
    parent TEXT REFERENCES categories (id)
           CHECK (parent IS NULL OR parent <> id)
) STRICT;

CREATE TABLE transactions (
    id         TEXT PRIMARY KEY,
    date       TEXT NOT NULL,
    kind       TEXT NOT NULL,
    payee      TEXT,
    note       TEXT,
    deleted_at TEXT
) STRICT;

CREATE TABLE postings (
    transaction_id TEXT NOT NULL
                   REFERENCES transactions (id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    target_kind    TEXT NOT NULL,
    target_id      TEXT,
    amount_minor   INTEGER NOT NULL,
    PRIMARY KEY (transaction_id, position),
    CHECK (
        (target_kind = 'opening' AND target_id IS NULL)
        OR (target_kind IN ('account', 'category') AND target_id IS NOT NULL)
    )
) STRICT;

CREATE TABLE budgets (
    category       TEXT NOT NULL REFERENCES categories (id),
    effective_from TEXT NOT NULL,
    amount_minor   INTEGER NOT NULL CHECK (amount_minor > 0),
    PRIMARY KEY (category, effective_from)
) STRICT;

CREATE INDEX postings_target ON postings (target_kind, target_id);
CREATE INDEX transactions_live ON transactions (deleted_at, date);

INSERT INTO schema_migrations (version, applied_at)
    VALUES (1, '2026-11-12T00:00:00Z');

INSERT INTO metadata (key, value)
    VALUES ('currency.code', 'PKR'), ('currency.exponent', '2');
";

/// Identifiers the fixture uses, so a test can name them.
pub fn account(value: u128) -> AccountId {
    AccountId::from_uuid(Uuid::from_u128(value))
}

/// Identifiers the fixture uses, so a test can name them.
pub fn category(value: u128) -> CategoryId {
    CategoryId::from_uuid(Uuid::from_u128(value))
}

/// Identifiers the fixture uses, so a test can name them.
pub fn transaction(value: u128) -> TransactionId {
    TransactionId::from_uuid(Uuid::from_u128(value))
}

/// Writes a version-one dataset at `path`: one account, one category and one
/// expense of 1,250.00.
pub fn write_version_one_dataset(path: &Path) {
    let raw = rusqlite::Connection::open(path).expect("the file is a database");
    raw.execute_batch(V1).expect("the v1 schema is written");

    let account = account(1).as_uuid().to_string();
    let category = category(1).as_uuid().to_string();
    let transaction = transaction(1).as_uuid().to_string();
    raw.execute(
        "INSERT INTO accounts (id, name, kind, currency_code, currency_exponent, archived)
         VALUES (?1, 'Pass-through', 'bank', 'PKR', 2, 0)",
        [account.as_str()],
    )
    .expect("the account is written");
    raw.execute(
        "INSERT INTO categories (id, name, kind, parent)
         VALUES (?1, 'Food', 'expense', NULL)",
        [category.as_str()],
    )
    .expect("the category is written");
    raw.execute(
        "INSERT INTO transactions (id, date, kind, payee, note, deleted_at)
         VALUES (?1, '2026-08-03', 'expense', 'Sample Mart', 'weekly shop', NULL)",
        [transaction.as_str()],
    )
    .expect("the transaction is written");
    raw.execute(
        "INSERT INTO postings
             (transaction_id, position, target_kind, target_id, amount_minor)
         VALUES (?1, 0, 'account', ?2, -125000),
                (?1, 1, 'category', ?3, 125000)",
        params![transaction.as_str(), account.as_str(), category.as_str()],
    )
    .expect("the postings are written");
}
