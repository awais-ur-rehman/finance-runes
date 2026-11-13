//! SQLite persistence for the finance domain.
//!
//! This crate maps domain objects to rows and back. It holds no business rules:
//! if a rule matters it lives in `finance-core`, and the adapter persists what
//! the domain already accepted (ADR-0004).
//!
//! The file on disk is treated as untrusted input even though we wrote it. Every
//! column is parsed explicitly, and an unknown value is an error rather than a
//! silent default, because a defaulted kind or currency would corrupt totals
//! without anyone noticing.
//!
//! A save replaces the live part of the dataset inside one transaction. That is
//! simple enough to verify and fast enough at personal-finance sizes, and it
//! makes "a save either happened or it did not" true by construction.
//! Soft-deleted transactions are left untouched, so undo and history survive a
//! save.
//!
//! Backups are single files with no sidecars, and restoring one validates the
//! candidate before it replaces anything. Exports are plain CSV, so the data
//! outlives the engine as well as the binary (FR-083).

mod backup;
mod codec;
mod error;
mod export;
mod integrity;
mod schema;
mod store;

pub use backup::Backup;
pub use backup::KEEP_BACKUPS;
pub use backup::file_name;
pub use backup::is_backup_name;
pub use backup::prune;
pub use backup::restore;
pub use error::StoreError;
pub use export::Export;
pub use export::ExportFile;
pub use export::FORMAT_VERSION;
pub use export::MANIFEST;
pub use export::directory_name;
pub use integrity::IntegrityReport;
pub use store::Store;
