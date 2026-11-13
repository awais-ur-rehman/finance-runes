# finance-import

**Status:** in progress — the parser is done; the review use case lives in `finance-app` and the Rune screen is pending

Reading a bank statement into rows, and nothing else. No file access, no clock, no database: the caller reads bytes and passes them in, which is what makes the importer testable from a string literal.

## What exists

- `parse(bytes, &Profile, currency)` — decodes the file, locates the header, resolves the columns and returns a `Statement`.
- `Profile` — a saved mapping of header names to date, description, instrument, debit/credit (or one signed amount) and balance. `Profile::bank_statement_debit_credit()` is the confirmed profile for the owner's bank.
- `Statement` — the rows that became transactions, the opening balance, the rows that did not, the totals, the balance breaks and the within-file duplicates.
- `Statement::preview()` — the aggregate FR-032 asks for: row count, date range, total in and out, final balance, unparseable rows with line numbers, the first balance break with both balances, duplicates and a count of suspected transfers.
- `StatementRow::merchant()` and `::transfer_hint()` — the shortened card merchant and the "this may be a transfer" guess (FR-033). The hint is a [`TransferPattern`](../finance-core/src/rule.rs) from the domain, because a remembered rule is keyed on it (ADR-0007).
- `DuplicateKey` — reference number when the bank gives one, otherwise date + signed amount + normalised description (FR-035).

## Pipeline

```text
bytes ─► decode ─► find header ─► resolve columns ─► rows ─► Statement ─► Preview
                                                       │
                                                       └─► issues (line + reason), parsing continues
```

A structural failure stops the import: undecodable bytes, no header in the first 50 rows, a header missing a column the profile names, or a column named twice. A row-level failure does not: it becomes a `RowIssue` and the rest of the file is still previewed, because one mistyped line must not cost the user the other three hundred.

A row that could not be parsed also ends the running-balance chain. The next parsed row anchors the check with the balance the file states, so one bad row produces one issue rather than a break at every row after it.

## What the parser handles

- **Header below a preamble.** Found by column name, not by position, so title lines above the header are harmless.
- **Encoding.** UTF-8 with or without a byte-order mark, UTF-16 LE and BE with a mark. Anything else is refused rather than decoded with a guessed code page, because a wrong decode could change an amount.
- **Quoting.** A description with commas or escaped quotes is read as one field.
- **Amounts.** Thousands separators (Western and South Asian grouping), parentheses for negatives, an empty cell as zero, and a hard failure on a third decimal place in a two-decimal currency.
- **Dates.** `DD-Mon-YYYY` (the confirmed format), day-first numeric with `-` or `/`, and ISO. The profile chooses; nothing is guessed per row.
- **Opening balance.** The `** Opening Balance` marker row becomes the account's opening balance, never a zero-amount transaction (FR-030).
- **Running balance.** `previous - out + in == balance` on every row; the first break is named with both balances (FR-032).
- **Own-account transfers.** `P2P FT` / `IBFT` and `ATM CASH` produce a hint with the counterparty, so a transfer is not imported as income.

## Owns

- Turning statement bytes into rows with line numbers.
- Balance continuity and duplicate detection inside the file.
- Hints that make the review step fast: merchant, transfer, reference.

## Must not

- Touch the filesystem, the clock or the database. The caller supplies bytes and the currency.
- Assign categories or counter-accounts. That needs the user's ledger and their confirmation (FR-033), so it belongs to the review step.
- Treat a hint as a decision. The raw description is always kept.

## Rules

- Every row is accounted for exactly once: as a transaction, as the opening balance, or as an issue.
- Money is parsed once, by `Money::parse`, into integer minor units (ADR-0003).
- A parser bug must not panic on a malformed file.
- Tests use the synthetic fixture in [`docs/requirements/csv-import-profile.md`](../../docs/requirements/csv-import-profile.md). No real statement, name or account number may appear in a test.

## Why the `csv` crate

Quoting, escaped quotes, embedded commas and embedded newlines are easy to get subtly wrong, and a bank description can contain all of them. The crate is pure Rust, does no I/O of its own and is already in wide use. Writing the reader by hand would add a parser to review for no gain in behaviour.

## Testing

Unit tests live beside each module; end-to-end parsing is in [`tests/parse.rs`](tests/parse.rs). See [testing strategy](../../docs/engineering/testing-strategy.md).
