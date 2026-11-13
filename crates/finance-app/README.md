# finance-app

The application layer: the use cases that sit between the domain and whatever
drives it, with no I/O of their own.

`finance-core` answers "what is true" (a balance, a budget's state). `finance-import`
answers "what is in the file". This crate answers "what does the user do next",
holds the state of a half-finished action, and hands the result back as plain
data.

## Why a separate crate

A review screen is a sequence of decisions before anything is written: pick a
category for this row, mark that one as a transfer, skip the eight rows already
imported. That sequence is business logic, not painting. Putting it in the UI
would make it untestable without a window, and would let a Rune upgrade change
what gets written. Putting it in `finance-core` would drag the importer and its
CSV dependency into the domain.

The boundary is enforced by what the crate is given: the ledger arrives as a
parameter, dates come from the statement, and transaction identifiers come from a
closure the caller supplies. The crate reads no clock and generates no
randomness, so every test is deterministic.

## What it owns

| Module | Responsibility |
| --- | --- |
| `import` | The import review: one decision per parsed row, remembered rules, the plan, and one atomic commit |

## The import review

`ImportSession::new(account, statement, &ledger)` opens a review. Three rules
shape it:

1. **A duplicate is skipped by default** (FR-035). A row that matches a
   transaction already recorded for the account, or an earlier row in the same
   file, starts as a skip. The user can post it anyway; the default is a warning,
   not a gate.
2. **An undecided row is never guessed at.** Every other row starts undecided and
   a commit refuses to run until each one has a category or a transfer. Filing a
   row under a category nobody chose is worse than asking.
3. **A commit is all or nothing** (FR-034). Rows are built first and handed to
   `Ledger::record_all`, so a failure half way through leaves no partial import.

Deciding a row teaches the session (FR-033): a card row remembers its merchant,
and a cash withdrawal or peer transfer remembers its pattern, never the whole
description. A description can carry an account number, and a rule keyed on one
would both stop matching the next transfer and copy the number into later rows.
Later rows that look the same are preselected; a row the user decided is never
overwritten. `clear_rules` returns the rows a rule decided to undecided and keeps
the user's own choices.

`plan()` is what the review screen shows before writing: rows, duplicates, rows
to post, rows skipped, rows undecided, transfers, and the in and out totals for
the rows that would actually be posted. `commit` returns the same numbers for
what it wrote.

## What it must never do

- Touch the filesystem, the network, the clock or a random number generator.
- Format money for display; that happens at the UI edge.
- Decide a category from a description. The importer offers a merchant hint and a
  transfer hint; the user, or a rule the user taught, decides.

## Tests

`src/import/tests.rs` runs the confirmed statement shape end to end: duplicate
detection against the ledger and within the file, rule learning and clearing,
plan totals, the opening balance, a refused batch leaving the ledger untouched,
and committing twice. Fixtures are synthetic; see
`docs/requirements/csv-import-profile.md`.
