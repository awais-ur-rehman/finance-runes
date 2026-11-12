//! Backups and restore.
//!
//! A backup is one self-contained file: `VACUUM INTO` writes a consistent copy
//! without needing the live file to be closed, and the copy is switched out of
//! WAL mode so it has no sidecar files to lose (FR-081). The name carries the
//! creation time and schema version, which makes a directory of backups readable
//! without opening any of them.
//!
//! Restore is the most dangerous operation in the app: it replaces the only copy
//! of the user's history. It therefore validates the candidate first — schema
//! version and a full integrity check — and only then writes anything, through a
//! temporary file in the same directory so the swap is a single rename (FR-082).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, params};
use time::{OffsetDateTime, UtcOffset};

use crate::error::StoreError;
use crate::integrity;
use crate::schema;

/// How many automatic backups are kept (FR-080).
pub const KEEP_BACKUPS: usize = 10;

/// A backup that was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backup {
    path: PathBuf,
    created_at: OffsetDateTime,
    schema_version: u32,
}

impl Backup {
    pub(crate) fn new(path: PathBuf, created_at: OffsetDateTime, schema_version: u32) -> Self {
        Self {
            path,
            created_at,
            schema_version,
        }
    }

    /// Where the backup was written.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The moment it was taken, as recorded in its name.
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// The schema version it holds.
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

/// The file name for a backup taken at `at` from schema `schema_version`.
///
/// Fixed-width UTC, so sorting the names sorts the backups by age. The schema
/// version is in the name as well as inside the file, so a backup from a newer
/// build is obvious before anything is opened.
pub fn file_name(at: OffsetDateTime, schema_version: u32) -> String {
    let at = at.to_offset(UtcOffset::UTC);
    format!(
        "finance-{:04}{:02}{:02}T{:02}{:02}{:02}Z-v{}.db",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second(),
        schema_version
    )
}

/// True when `name` is a backup this app wrote.
///
/// Pruning relies on this to decide what it may delete, so it must be strict: a
/// false positive would delete a file the user put in the backup directory.
pub fn is_backup_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("finance-") else {
        return false;
    };
    let Some(rest) = rest.strip_suffix(".db") else {
        return false;
    };
    let Some((stamp, version)) = rest.rsplit_once("-v") else {
        return false;
    };

    is_stamp(stamp) && !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_stamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 16
        && bytes[8] == b'T'
        && bytes[15] == b'Z'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 8 || index == 15 || byte.is_ascii_digit())
}

/// Deletes the oldest backups in `directory`, keeping the newest `keep`.
///
/// Only files whose name matches [`is_backup_name`] are candidates. Returns how
/// many were removed.
///
/// # Errors
///
/// Returns [`StoreError::Io`] if the directory cannot be read or a file cannot
/// be removed.
pub fn prune(directory: &Path, keep: usize) -> Result<usize, StoreError> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_backup_name)
        {
            names.push(path);
        }
    }

    names.sort();
    let excess = names.len().saturating_sub(keep);
    let mut removed = 0;
    for path in names.into_iter().take(excess) {
        std::fs::remove_file(path)?;
        removed += 1;
    }

    Ok(removed)
}

/// Replaces `dataset` with the backup at `backup`.
///
/// Validates the backup first, then writes a normalised copy next to the dataset
/// and renames it into place, so a failure before the rename leaves the current
/// dataset untouched.
///
/// The caller must close any [`Store`](crate::Store) on `dataset` first; the
/// sidecar files of the old dataset are removed, which is only safe when nothing
/// has it open.
///
/// # Errors
///
/// Returns [`StoreError::UnsupportedSchema`] for a backup from a newer build,
/// [`StoreError::Integrity`] when it fails its integrity check,
/// [`StoreError::Corrupt`] when it holds no schema at all, and
/// [`StoreError::Database`] when it cannot be read.
pub fn restore(backup: &Path, dataset: &Path) -> Result<u32, StoreError> {
    let schema_version = validate(backup)?;

    let temporary = dataset.with_extension("restore");
    if temporary.exists() {
        std::fs::remove_file(&temporary)?;
    }
    vacuum_into(backup, &temporary)?;

    // A leftover write-ahead log belongs to the old dataset and would be applied
    // to the new file, which is exactly how a restore destroys data.
    remove_sidecars(dataset);
    std::fs::rename(&temporary, dataset)?;

    Ok(schema_version)
}

/// Checks that a file is a backup this build can restore, and returns its schema
/// version.
fn validate(backup: &Path) -> Result<u32, StoreError> {
    let connection = Connection::open_with_flags(backup, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let version = schema::current_version(&connection)?;

    if version > schema::SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema {
            found: version,
            supported: schema::SCHEMA_VERSION,
        });
    }
    if version == 0 {
        return Err(StoreError::Corrupt {
            table: "schema_migrations",
            detail: "the file holds no schema".to_owned(),
        });
    }

    let report = integrity::check(&connection)?;
    if !report.is_ok() {
        return Err(StoreError::Integrity {
            problems: report.problems().to_vec(),
        });
    }

    Ok(version)
}

/// Copies `source` into `target`, which must not exist.
///
/// `VACUUM INTO` is SQLite's own consistent-copy mechanism: it reads through the
/// write-ahead log, so the copy is a complete dataset rather than a byte-level
/// snapshot that might be missing the tail (ADR-0004).
pub(crate) fn vacuum_into(source: &Path, target: &Path) -> Result<(), StoreError> {
    let text = target
        .to_str()
        .ok_or_else(|| StoreError::InvalidPath(target.to_path_buf()))?;
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.execute("VACUUM INTO ?1", params![text])?;
    Ok(())
}

/// Removes `<dataset>-wal` and `<dataset>-shm` if they exist.
fn remove_sidecars(dataset: &Path) {
    for suffix in ["-wal", "-shm"] {
        let mut name = OsString::from(dataset.as_os_str());
        name.push(suffix);
        let path = PathBuf::from(name);
        if path.exists() {
            // Best effort: a failure here is reported by the rename that follows,
            // which cannot succeed while another process holds the file.
            let _ = std::fs::remove_file(path);
        }
    }
}
