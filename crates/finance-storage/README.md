# finance-storage

**Status:** in progress — schema v3, migrations, round-trip, soft delete, backup, restore and CSV export done; the pre-migration backup is pending

Persistence, and nothing else. Maps domain objects to rows and back, applies migrations, produces consistent backups.

## What exists

- `Store::open` / `open_in_memory` — opens or creates a dataset, refuses a newer schema version, fixes the currency at open time.
- Schema v3 — `schema_migrations`, `metadata`, `accounts`, `categories`, `transactions`, `postings`, `budgets`, `rules`, all `STRICT`, with migrations applied one transaction at a time. v2 adds provenance (`source`, `source_profile`) to `transactions` (FR-036); v3 adds the remembered rules (FR-033).
- `load` / `save` — the whole ledger in and out; a save replaces the live dataset inside one transaction and leaves soft-deleted rows alone.
- `soft_delete_transaction` / `restore_transaction` / `deleted_transactions` — the undo trail (FR-014, Q-34).
- `backup_to` / `backup_into` / `prune` — `VACUUM INTO` copies, named from their timestamp and schema version, ten kept (FR-080, FR-081, FR-086).
- `restore` — validates version and integrity, then swaps the file through a temporary copy (FR-082).
- `export_into` — every table to plain CSV files plus a manifest, in one new directory (FR-083).
- `integrity_check` — reports problems and changes nothing (FR-085).
- `needs_backup` — whether anything was written since the last backup, so exit can skip an empty one.

## On-disk layout

One `dataset.db` per dataset. While the app has it open, SQLite keeps `dataset.db-wal` and `dataset.db-shm` beside it; those files belong to the dataset, so copy the directory rather than the `.db` alone, or close the app first.

Backups are named `finance-YYYYMMDDTHHMMSSZ-v<schema>.db`, so a directory of them sorts by age and shows the schema version without opening anything. Each is a single file in rollback-journal mode: no sidecars, readable by any SQLite client.

| Table | Contents |
| --- | --- |
| `schema_migrations` | One row per applied migration |
| `metadata` | Dataset currency, as a code and an exponent |
| `accounts` | Real places money sits, with an archived flag |
| `categories` | Two-level tree; `parent` is null at the top |
| `transactions` | Date, kind, payee, note, provenance (`source` + `source_profile`), and `deleted_at` for soft deletes |
| `postings` | One row per leg: `account`, `category` or `opening`, plus integer minor units |
| `budgets` | A limit per category, in force from a `YYYY-MM` period |
| `rules` | A remembered merchant or transfer pattern and what a matching row becomes |

Money is always an integer count of minor units plus a currency code, never a float (ADR-0003). The opening-balance target has no identifier, which is why `postings` carries a discriminator and a nullable identifier instead of a foreign key.

## Export format

`Store::export_into(parent, at)` writes one directory named `finance-export-YYYYMMDDTHHMMSSZ`, holding one file per table plus `manifest.csv`. The files are written into `<name>.partial` and the directory is renamed when every file is complete, so an interrupted export is never mistaken for a finished one.

| File | Columns |
| --- | --- |
| `accounts.csv` | `id,name,kind,currency_code,currency_exponent,archived` |
| `categories.csv` | `id,name,kind,parent_id` |
| `transactions.csv` | `id,date,kind,payee,note,source,source_profile,deleted_at` |
| `postings.csv` | `transaction_id,position,target_kind,target_id,amount,currency_code` |
| `budgets.csv` | `category_id,effective_from,amount,currency_code` |
| `rules.csv` | `key_kind,merchant,pattern,counterparty,target_kind,target_id` |
| `manifest.csv` | `key,value` |

- **Money is plain decimal text** (`-1250.00`, `1.234`), beside its currency code, so a spreadsheet reads it without knowing about minor units. The exponent is in the manifest and in every account row.
- **Soft-deleted transactions are included**, with their deletion time (Q-34). The export is the copy the user keeps; hiding a deletion marker would make it lossy.
- **The manifest** carries `format_version`, `schema_version`, `created_at` (RFC 3339), `currency_code`, `currency_exponent`, and one `<file>.rows` entry per file, so a reader can check that nothing was truncated.
- **An empty optional field is an empty string**, never the text `null`.
- A second export at the same moment is refused rather than overwriting the first.

The layout is versioned by `FORMAT_VERSION` and does not depend on SQLite: the data can be read by any CSV reader, and a future engine would export the same files.

## Owns

- The SQLite adapter (ADR-0004), one file per dataset.
- Schema definitions and the migration runner, versioned and forward-only.
- Repository implementations behind traits defined by the application layer.
- Consistent backup (`VACUUM INTO`), restore with version and integrity checks.
- The documented CSV export, independent of the engine (FR-083).
- Integrity verification and the on-disk layout documentation.

## Must not

- Contain business rules. If a rule matters, it belongs in `finance-core`.
- Decide what a valid transaction is; it persists what the domain already validated.
- Leak SQL or row types into the UI.

## Rules

- Every write is a single transaction; imports and migrations are all-or-nothing.
- Validate on read: the file on disk is untrusted input, even if we wrote it.
- Never open a dataset with a newer schema version — refuse with a clear message (NFR-032).
- Tests use a real SQLite file in a temp directory, never a mock of the database.
- Keep `std::fs` and `PathBuf` out of the persistence API where possible, so a `wasm32` target could back it with browser storage later (a Rune convention worth keeping).

## Testing

Round-trip, migration-from-every-version, atomicity and crash-injection tests. See [testing strategy](../../docs/engineering/testing-strategy.md).
