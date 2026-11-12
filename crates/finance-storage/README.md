# finance-storage

**Status:** planned — no code yet

Persistence, and nothing else. Maps domain objects to rows and back, applies migrations, produces consistent backups.

## Owns

- The SQLite adapter (ADR-0004), one file per dataset.
- Schema definitions and the migration runner, versioned and forward-only.
- Repository implementations behind traits defined by the application layer.
- Consistent backup (`VACUUM INTO`), restore with version and integrity checks.
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
