//! The on-disk schema and the forward-only migration runner.
//!
//! Each migration is applied in its own transaction together with the row that
//! records it, so a crash leaves the dataset at a version it actually reached
//! (NFR-004). A dataset from a newer build is refused rather than opened: this
//! build does not know what the extra columns mean, and guessing would write
//! over data it cannot see (NFR-032).
//!
//! Before an upgrade the dataset is copied to `<name>.pre-v<version>.db` beside
//! it, so an upgrade that turns out to be wrong is a file rename away from being
//! undone (FR-084). The copy is named after the version it protects rather than
//! the time it was taken, so there is one per upgrade and a repeated attempt
//! replaces it instead of filling the directory.
//!
//! Money is stored as integer minor units plus a currency code, never as a
//! float (ADR-0003). Tables are `STRICT`, so SQLite rejects a text value in an
//! integer column instead of silently converting it.

use std::path::{Path, PathBuf};

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

/// Opens a connection for use: durability settings, foreign keys, a
/// pre-migration copy, then migrations.
///
/// `dataset` is the file behind `connection`, or `None` for an in-memory
/// dataset, which has nothing to copy. Returns the copy that was taken, if the
/// open upgraded the file (FR-084).
pub(crate) fn prepare(
    connection: &mut Connection,
    dataset: Option<&Path>,
) -> Result<Option<PathBuf>, StoreError> {
    // WAL survives a crash mid-write and lets a reader run during a write.
    // `FULL` costs one fsync per commit, which is irrelevant at this data size
    // and buys the strongest durability SQLite offers (ADR-0004, NFR-003).
    connection.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = FULL;")?;

    let copy = match dataset {
        Some(path) => backup_before_migration(connection, path)?,
        None => None,
    };
    migrate(connection)?;

    Ok(copy)
}

/// Copies the dataset before a migration changes it (FR-084).
///
/// The copy is a `VACUUM INTO` snapshot switched out of WAL mode, so it is one
/// self-contained file. A copy from an earlier attempt is replaced: migrations
/// are transactional, so the dataset is still at the old version here, and a
/// fresh copy is at least as good as a stale one.
fn backup_before_migration(
    connection: &Connection,
    dataset: &Path,
) -> Result<Option<PathBuf>, StoreError> {
    let version = current_version(connection)?;
    if version == 0 || version >= SCHEMA_VERSION {
        return Ok(None);
    }

    let target = pre_migration_path(dataset, version)?;
    let staging = target.with_extension("partial");
    if staging.exists() {
        std::fs::remove_file(&staging)?;
    }

    let written = copy_into(connection, &staging).and_then(|()| {
        switch_to_rollback_journal(&staging)?;
        std::fs::rename(&staging, &target)?;
        Ok(())
    });
    if written.is_err() {
        // A half-written copy must not be mistaken for a usable backup.
        let _ = std::fs::remove_file(&staging);
    }
    written?;

    Ok(Some(target))
}

/// Where the copy taken before upgrading from `version` is written.
fn pre_migration_path(dataset: &Path, version: u32) -> Result<PathBuf, StoreError> {
    let name = dataset
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| StoreError::InvalidPath(dataset.to_path_buf()))?;
    Ok(dataset.with_file_name(format!("{name}.pre-v{version}.db")))
}

/// SQLite's own consistent copy: it reads through the write-ahead log, so the
/// copy is a complete dataset rather than a byte-level snapshot (ADR-0004).
fn copy_into(connection: &Connection, target: &Path) -> Result<(), StoreError> {
    let text = target
        .to_str()
        .ok_or_else(|| StoreError::InvalidPath(target.to_path_buf()))?;
    connection.execute("VACUUM INTO ?1", params![text])?;
    Ok(())
}

/// Leaves the copy with no write-ahead log of its own to lose.
fn switch_to_rollback_journal(path: &Path) -> Result<(), StoreError> {
    let copy = Connection::open(path)?;
    copy.query_row("PRAGMA journal_mode = DELETE", [], |_| Ok(()))?;
    Ok(())
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
