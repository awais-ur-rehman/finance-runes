# ADR-0004 — Local-first storage in a single SQLite file

**Status:** accepted (2026-11-12)
**Date:** 2026-11-12
**Deciders:** project owner
**Supersedes:** —

## Context

The dataset is one person's complete financial history: tens of thousands of transactions, categories, budgets, rules and settings. Requirements that bear on storage:

- **Local-first, no server.** The app must work offline, forever.
- **Durability.** A crash mid-write must not corrupt the dataset (NFR-003).
- **Portability.** The user must be able to copy, back up and export everything (FR-080, FR-081).
- **Migrations.** The schema will change; upgrades must be automatic and reversible-by-backup (FR-082).
- **Query-heavy UI.** Reports, search, budgets and reconciliation are all queries over the same data (NFR-012).
- **Single process.** No concurrent writers; one user, one window (mostly).

## Options considered

| Option | Pros | Cons |
| --- | --- | --- |
| **SQLite (bundled)** | ACID, single file, mature, `VACUUM INTO` for consistent backup, SQL for reports, universally understood, easy to inspect with any tool | FFI dependency (or `rusqlite` bundled); schema migrations are ours to write; a `.db` is opaque without a client |
| Embedded KV (redb, sled, fjall) | Pure Rust, no FFI, simple API, fast point reads | No query language; reports and ad-hoc search become hand-written indexes; migrations are manual |
| Append-only log + index | Perfect audit trail, trivial durability semantics | Compaction, indexing and querying are all bespoke; slow to build correctly |
| JSON files | Human-readable, no dependency | Rewrite-the-world writes, no atomicity, no indexes; fine for settings, not for a ledger |

## Decision

Store the dataset in a **single SQLite database file** (bundled SQLite via `rusqlite`), accessed only through the `finance-storage` crate behind repository traits defined by the application layer.

Rules:

1. **One file per dataset**, in a user-chosen directory. Attachments live beside it in a managed folder (Q-10).
2. **WAL mode**, `synchronous = NORMAL` or `FULL` (decide with a benchmark), foreign keys on.
3. **Every write is one transaction.** No partial imports (FR-034), no partial migrations (NFR-004).
4. **Migrations are versioned and forward-only**, recorded in a `schema_version` table, each with a test that runs it from every historical schema.
5. **Backup uses SQLite's own consistent-copy mechanism** (`VACUUM INTO`), never a file copy of a live database.
6. **Integrity is checkable** without modifying data (`PRAGMA integrity_check`, NFR-084).
7. **The storage crate contains no business rules.** It maps rows to domain types and back.
8. **The export format is independent of SQLite.** CSV per table plus a manifest (FR-081), so the data survives the engine.

## Consequences

**Positive** — we get durability, transactions and a query engine for free; backups are a solved problem; contributors can inspect the file with standard tools; reporting queries stay declarative.

**Negative** — we take an FFI dependency and must vendor/pin it for reproducible builds across three platforms. Schema evolution is real work. SQLite types are looser than Rust's, so the adapter must validate on read and never trust the file.

**Neutral** — a future engine swap is possible but would mean rewriting the adapter and a migration path; the repository trait keeps that bounded.

## Follow-ups

- [ ] Benchmark WAL + `synchronous` settings against NFR-010/NFR-014.
- [ ] Write the schema-version and migration harness before the first table.
- [ ] Add a crash-injection test that kills the process during a write loop.
- [ ] Document the on-disk layout in `docs/engineering/` so a user can recover data by hand.
