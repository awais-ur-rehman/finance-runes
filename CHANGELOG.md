# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entries are written for the person using the app, not for the person who wrote the code. Internal refactors without user-visible effect are omitted unless they change behaviour or performance noticeably.

## [Unreleased]

### Added

- Repository scaffold: project brief, requirement documents, architecture decision records, engineering standards, and issue/PR templates.
- Toolchain pinned to Rust 1.99.1 to match the Rune `dev` toolchain.
- Import review use case: a parsed statement becomes one decision per row — a category, a transfer to another of your own accounts, or a skip — with duplicates skipped by default, remembered merchant and transfer choices, and a single atomic commit.
- Remembered rules now outlive the import that taught them: the merchant → category and pattern → transfer choices are saved with your data, so the next import preselects the rows you have already filed once.
- Export all data to plain CSV files plus a manifest, readable without this app and independent of the database engine.
- Before an upgrade, a copy of the dataset as it was is kept beside it, so an upgrade that turns out to be wrong can be undone by renaming one file.
- Provenance: every transaction records whether you typed it in or imported it, and which import profile read it, so the register can say where a row came from.

### Notes

- No UI yet. The domain, storage, import and application crates are in place; see the [roadmap](README.md#roadmap).

<!--
## [0.1.0] - YYYY-MM-DD

### Added
- ...

### Changed
- ...

### Fixed
- ...

### Security
- ...
-->
