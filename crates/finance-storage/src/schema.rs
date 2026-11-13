//! The on-disk schema and the forward-only migration runner.
//!
//! Each migration is applied in its own transaction together with the row that
//! records it, so a crash leaves the dataset at a version it actually reached
//! (NFR-004). A dataset from a newer build is refused rather than opened: this
//! build does not know what the extra columns mean, and guessing would write
//! over data it cannot see (NFR-032).
//!
//! Money is stored as integer minor units plus a currency code, never as a
//! float (ADR-0003). Tables are `STRICT`, so SQLite rejects a text value in an
//! integer column instead of silently converting it.

use rusqlite::{Connection, params};
use time::OffsetDateTime;

use crate::StoreError;
use crate::codec;

const V1: &str = r#"
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
    parent TEXT REFERENCES categories (id) CHECK (parent IS NULL OR parent <> id)
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
    transaction_id TEXT NOT NULL REFERENCES transactions (id) ON DELETE CASCADE,
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
"#;

/// Provenance: where a transaction came from (FR-036).
///
/// The default is `manual` so the rows already in a v1 dataset — all of which
/// were typed in or written before provenance existed — stay truthful. The
/// profile is kept beside the kind rather than in a separate table, because it
/// is a name the import wrote, not a row with its own identity.
const V2: &str = r#"
ALTER TABLE transactions ADD COLUMN source TEXT NOT NULL DEFAULT 'manual'
    CHECK (source IN ('manual', 'imported'));
ALTER TABLE transactions ADD COLUMN source_profile TEXT;
"#;

/// Remembered classification rules (FR-033).
///
/// A rule is keyed on the value the importer derived, never on a description,
/// and the key is the identity: re-teaching a row replaces the row. SQLite
/// cannot make a composite primary key out of nullable columns without letting
/// duplicates through, so an absent part of the key is an empty string and the
/// `CHECK` keeps the two key shapes apart. A merchant name is never blank and a
/// counterparty is blank only when the description named nobody, so the sentinel
/// cannot collide with a real value.
const V3: &str = r#"
CREATE TABLE rules (
    key_kind     TEXT NOT NULL CHECK (key_kind IN ('merchant', 'transfer')),
    merchant     TEXT NOT NULL DEFAULT '',
    pattern      TEXT NOT NULL DEFAULT '',
    counterparty TEXT NOT NULL DEFAULT '',
    target_kind  TEXT NOT NULL CHECK (target_kind IN ('category', 'account')),
    target_id    TEXT NOT NULL,
    PRIMARY KEY (key_kind, merchant, pattern, counterparty),
    CHECK (
        (key_kind = 'merchant'
            AND merchant <> '' AND pattern = '' AND counterparty = '')
        OR (key_kind = 'transfer'
            AND merchant = ''
            AND pattern IN ('cash_withdrawal', 'peer_transfer')
            AND (pattern = 'peer_transfer' OR counterparty = ''))
    )
) STRICT;
"#;

/// Every migration, oldest first. The index plus one is the schema version.
const MIGRATIONS: &[&str] = &[V1, V2, V3];

/// The version this build writes and the highest it can open.
pub(crate) const SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

/// Opens a connection for use: durability settings, foreign keys, migrations.
pub(crate) fn prepare(connection: &mut Connection) -> Result<(), StoreError> {
    // WAL survives a crash mid-write and lets a reader run during a write.
    // `FULL` costs one fsync per commit, which is irrelevant at this data size
    // and buys the strongest durability SQLite offers (ADR-0004, NFR-003).
    connection.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = FULL;")?;
    migrate(connection)
}

/// The schema version recorded in the dataset, or zero for an empty file.
pub(crate) fn current_version(connection: &Connection) -> Result<u32, StoreError> {
    let present: bool = connection.query_row(
        "SELECT EXISTS (
             SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'
         )",
        [],
        |row| row.get(0),
    )?;
    if !present {
        return Ok(0);
    }

    let version: Option<i64> =
        connection.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
    let version = version.unwrap_or(0);
    u32::try_from(version).map_err(|_| StoreError::Corrupt {
        table: "schema_migrations",
        detail: format!("version {version} is not a positive number"),
    })
}

/// Applies every migration the dataset has not seen.
fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
    let current = current_version(connection)?;
    if current > SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema {
            found: current,
            supported: SCHEMA_VERSION,
        });
    }

    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = index as u32 + 1;
        if version <= current {
            continue;
        }

        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![version, codec::timestamp(OffsetDateTime::now_utc())?],
        )?;
        transaction.commit()?;
    }

    Ok(())
}
