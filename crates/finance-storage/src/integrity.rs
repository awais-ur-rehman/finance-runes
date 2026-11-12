//! Integrity checks that never modify the dataset.
//!
//! `PRAGMA integrity_check` reads every page and reports what it finds; it does
//! not repair anything. That is the point: a check that "fixes" a file is a
//! check you cannot trust before a restore (FR-085).

use rusqlite::Connection;

use crate::StoreError;

/// What an integrity check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityReport {
    problems: Vec<String>,
}

impl IntegrityReport {
    /// True when the dataset is structurally sound.
    pub fn is_ok(&self) -> bool {
        self.problems.is_empty()
    }

    /// One line per problem, as SQLite reported it. Empty when the dataset is
    /// sound.
    pub fn problems(&self) -> &[String] {
        &self.problems
    }
}

/// Runs `PRAGMA integrity_check` on an open connection.
pub(crate) fn check(connection: &Connection) -> Result<IntegrityReport, StoreError> {
    let mut statement = connection.prepare("PRAGMA integrity_check")?;
    let lines: Vec<String> = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    // A sound database answers with exactly one row, "ok".
    let problems = lines
        .into_iter()
        .filter(|line| line != "ok")
        .collect::<Vec<_>>();

    Ok(IntegrityReport { problems })
}
