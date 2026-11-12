# Testing strategy

**Status:** active from the first test
**Last updated:** 2026-11-12

Money bugs are not cosmetic. The test suite exists to make it impossible to ship a wrong balance without noticing.

## Shape of the suite

```text
        ▲  few, slow, expensive
        │  end-to-end / headless UI renders
        │  importer golden files
        │  storage integration (real SQLite, temp dir)
        │  property tests (money, ledger, allocation)
        ▼  many, fast, pure
           domain unit tests
```

Target distribution: ~70% domain unit and property tests, ~20% storage/import integration, ~10% UI and end-to-end. Not a coverage number — a *risk* allocation. The domain is where the risk is.

## Layers

### Domain (`finance-core`)

Pure, deterministic, no I/O. Runs in milliseconds, on every save.

- Unit tests per invariant: postings sum to zero, split parts sum to the total, transfer requires two accounts, currency mismatch rejected.
- Property tests (`proptest`) for anything with arithmetic:
  - random postings → derived balance equals an independent integer sum,
  - split an amount into *n* parts → parts sum to the amount, for all *n* and amounts,
  - currency conversion round-trip within one minor unit,
  - date ranges: `balance_as_of(d)` is monotonic in `d`.
- Table-driven tests for currency exponents (JPY 0, USD 2, KWD 3).

### Storage (`finance-storage`)

Real SQLite in a temp directory — never a mock of the database.

- Round-trip: write a domain object, read it back, compare structurally.
- Migration tests: build the schema at every historical version, migrate to current, assert the data survives.
- Atomicity: force a failure mid-transaction, assert nothing was written.
- Crash injection: kill the process during a write loop, reopen, assert integrity.
- Integrity: `PRAGMA integrity_check` passes after a torture sequence.

### Importers

- One golden fixture per bank format, with a checked-in expected result. Fixtures are synthetic or irreversibly anonymised.
- Failure cases: empty file, header only, malformed date, unknown encoding, 10 MB file, CRLF vs LF, quoted commas, BOM, negative-in-parentheses amounts.
- Dedupe tests: same file twice imports nothing the second time; a re-posted row is surfaced as a possible duplicate, not silently skipped.
- Fuzz targets (`cargo-fuzz`) for each parser; a panic is a failing test.

### UI (`finance-ui`)

Rune can run headless, render to PNG, dump the element tree and accept scripted input. Use it.

- **Element-tree assertions** for structure: the register shows one row per transaction; the budget row shows budgeted/actual/remaining.
- **Golden renders** per screen at 100% and 150% scale, with a small tolerance and an explicit update command. A diff is a review conversation, not an auto-accept.
- **Keyboard-only scripts** for primary flows (NFR-040).
- **Frame-time traces** for the scroll and search budgets (NFR-011, NFR-012).
- **No network**: run the UI suite with a blocking harness and assert zero outbound connections (NFR-020).

### End-to-end

A handful of journeys, not a hundred: create an account, import a statement, reconcile a month, set a budget, view a report, back up and restore. Each one uses a temp dataset and asserts the final state through the public API.

## Rules

1. **A bug fix starts with a failing test.** Name it after the bug.
2. **Tests are deterministic.** No wall clock (inject a fixed clock), no network, no shared global state, no reliance on test order.
3. **No sleeps for synchronisation.** Wait on a condition or an explicit signal.
4. **Assertions are specific.** `assert_eq!(balance, Money::from_minor(-12000, USD))`, not `assert!(balance.is_negative())`.
5. **One behaviour per test.** A test with three acts is three tests.
6. **Test data is synthetic.** `Example Bank`, invented payees, amounts that are obviously fake.
7. **Snapshots are reviewed.** A changed golden render or snapshot is part of the PR diff and must be explained.
8. **Flaky tests are removed or fixed immediately.** A quarantined test is a deleted test with extra steps.

## Naming

```rust
#[test]
fn transfer_between_same_account_is_rejected() { }

#[test]
fn split_parts_must_sum_to_transaction_total() { }

#[test]
fn import_skips_rows_already_present_by_fingerprint() { }
```

Pattern: `<subject>_<condition>_<expected>`. A reader should know what broke from the failing test name alone.

## Coverage

Coverage is a smoke alarm, not a target. We aim for **high coverage of `finance-core` invariants** and accept lower coverage in UI glue. A line covered by a test that asserts nothing is worse than an uncovered line, because it lies.

## Running

```bash
cargo test --workspace                 # fast suite, every save
cargo test --workspace -- --nocapture  # when debugging
cargo test -p finance-core --release   # property tests with more cases
cargo fuzz run import_csv              # parsers, when touching them
```

CI runs the fast suite on every push, the storage and UI suites on every PR, and the fuzzers on a schedule.
