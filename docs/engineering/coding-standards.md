# Coding standards

**Status:** active from the first line of code
**Last updated:** 2026-11-12

Rust's compiler catches a great deal. These standards exist for what it cannot: money correctness, boundaries between layers, and code another person can review.

## Formatting and lints

- `cargo fmt` is the only formatter; no manual alignment. Rustfmt defaults, 4-space indent, 100-column max width.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass. Fix the cause, do not `#[allow]` it.
- Allowed lints must be justified in a comment on the `allow` line.
- Every crate enables:

```rust
#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]
#![deny(unsafe_op_in_unsafe_fn)]
```

- `unsafe` requires a `// SAFETY:` comment stating the invariant and why it holds. It is banned in `finance-core`.

## Naming

| Thing | Convention | Example |
| --- | --- | --- |
| Types, traits, enums | `UpperCamelCase`, noun | `Transaction`, `Money`, `AccountKind` |
| Functions, methods | `snake_case`, verb phrase | `record_transaction`, `balance_as_of` |
| Booleans | `is_`, `has_`, `can_` | `is_reconciled`, `can_delete` |
| Units in names | Always suffix the unit | `amount_minor`, `timeout_ms`, `rate_ppm` |
| Errors | `<Verb>Error` or `<Noun>Error` | `ImportError`, `CurrencyMismatch` |
| Conversions | `to_` / `as_` / `into_` per std semantics | `to_display_string`, `as_minor` |

No abbreviations except `id`, `db`, `tx`, `fx`, `ui`. `cnt`, `amt`, `acct` are banned.

## Types before comments

- Newtypes for domain identity: `AccountId`, `TransactionId` — never bare `u64` or `String`.
- Enums for modes; a struct of `Option`s plus booleans that encodes a mode is a bug. Follow Rune's rule: one `enum State` whose variants carry only the data valid in them, and one `step` function.
- Parse, don't validate: construct types that cannot be invalid. If a `Money` can be created with mismatched currency, the type is wrong.
- Prefer `NonEmpty<Vec<_>>`-style invariants over "the caller must ensure at least one".

## Errors

- Domain errors are typed enums with `thiserror`, one variant per failure the caller can act on. A single `DomainError(String)` is not an error type.
- `anyhow` is permitted only in `main.rs` and test helpers. Library crates expose concrete errors.
- `unwrap()` / `expect()` are forbidden outside tests. `expect` is acceptable in a `main` bootstrap with a message a user could act on.
- Never swallow an error: no `let _ =`, no `if let Ok(_)` without a comment explaining why the failure is irrelevant.
- Error messages say what failed and what to do, and never contain amounts, payees or file paths from the user's data.

## Money rules

1. No `f64` anywhere in `finance-core`. A `f64` in a money path is a review blocker.
2. All arithmetic is checked; overflow returns an error, never wraps.
3. Rounding is explicit: every division names its rounding mode.
4. Currency is always carried with the amount; there is no currency-less number.
5. Formatting for display happens at the UI edge, using the locale, never in the domain.

## Layering rules

- `finance-core` has no `std::fs`, `std::net`, `std::time::Instant::now`, `rand`, `log`, or UI imports. Enforced by a CI dependency check.
- The UI never writes to the database and never owns business rules. It sends commands and reads projections.
- A crate may only depend on crates to its left in the diagram in [architecture](../architecture/README.md).

## File and module shape

- **800 non-test lines per Rust file** is the ceiling (Rune's rule; it prevents god files and edit collisions). Split before you hit it, not after.
- Tests live in a sibling `tests.rs` or `*_tests.rs`, never inline beyond a handful of lines.
- A folder appears when three sibling files share a prefix; a folder over 40 source files needs splitting.
- `lib.rs` keeps `mod` and `pub use` lists one item per line, sorted.
- One public type per file is the default; group tightly coupled types (e.g. `Money` and its arithmetic errors) when splitting would only add indirection.

## Documentation

- Every `pub` item has a doc comment. It explains *why* and any invariant, not the signature in prose.
- Module-level `//!` comment states the module's responsibility and what it must not do.
- Comments explain non-obvious decisions. `// increment counter` is noise.
- A `TODO` includes an owner or an issue reference: `// TODO(#42): handle JPY rounding`.

## Performance

- Measure before optimising; a PR claiming a speedup includes numbers from `--release`.
- No I/O on the UI thread during a frame — no file reads, writes, stats or renames (Rune's rule; on Windows each call can block on antivirus).
- Long work runs off-thread and reports progress; the UI stays interactive.
- Bound every cache in entries and bytes. An unbounded cache is a leak with a nicer name.
- Build only what is visible: virtualise long lists, memoise rows that rarely change.

## Tests

- Every bug fix starts with a failing test that reproduces it.
- Test names describe behaviour: `transfer_between_same_account_is_rejected`.
- No test depends on the wall clock, the network, the user's locale, or another test's state.
- Fixtures are synthetic. See [data privacy](data-privacy-and-security.md).

## Commits and PRs

- Conventional Commits, imperative mood, lowercase after the type: `fix(import): skip rows with an empty date`.
- One logical change per commit; a refactor and a behaviour change are two commits.
- No commented-out code, no debug prints, no stray `dbg!` in a PR.

## Checklist before pushing

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --cached          # read it, end to end
```
