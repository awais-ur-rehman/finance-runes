# CSV import profile — bank statement

**Status:** shape confirmed from a real export; still missing the raw delimiter details
**Last updated:** 2026-11-12
**Answers:** Q-29
**Requirements:** FR-030 … FR-038

## 1. Why this document exists

Q-29 asked what the owner's bank CSV actually contains. It is now known, so the
importer and its saved profile can be built against the real shape instead of a
guess. This document records the *shape* only.

## 2. Privacy rule

- The owner's statement is **not** in this repository and must never be.
- Every example here is synthetic: the same shape, invented values, invented
  names, invented reference numbers.
- Tests and fixtures use the synthetic rows below. No real account number,
  balance, merchant or person's name may appear in a commit.
- Screenshots of a real statement are not committed either; they are read once
  and transcribed into this shape.

## 3. Column layout

Six columns, one header row:

| # | Header | Meaning | Required |
| --- | --- | --- | --- |
| 1 | `DATE` | Date the transaction was posted | yes |
| 2 | `PARTICULARS` | Free-text description: the merchant or counterparty | yes |
| 3 | `INST. NO.` | Instrument number, filled only for cheque instruments | no |
| 4 | `DEBIT` | Money out, `0` when the row is a credit | yes |
| 5 | `CREDIT` | Money in, `0` when the row is a debit | yes |
| 6 | `BALANCE` | Account balance after the row | yes |

Debit and credit are **separate columns**, and exactly one of them is non-zero
on any real row. This is the second branch of FR-030.

## 4. Value formats

| Field | Format | Example (synthetic) |
| --- | --- | --- |
| Date | `DD-Mon-YYYY` | `12-Aug-2026` |
| Amount | Thousands separators, two decimals, no currency symbol | `4,500.00` |
| Empty field | Empty string, not `0` | `INST. NO.` is empty on card rows |
| Text | Free text, may contain commas and be long | see §5 |

Amounts never carry a sign: direction comes from which column is populated.

## 5. Structural quirks

Four things in this file are not "just rows", and each needs explicit handling.

### 5.1 Opening balance row

The first data row is a marker, not a transaction:

```text
12-Aug-2026,** Opening Balance,,0,0,1250.00
```

Zero debit, zero credit, and the account's starting balance in `BALANCE`.
It is imported as one opening-balance entry (FR-030), which the domain already
models. It must not become a zero-amount transaction, and it must not be counted
as income or spending.

### 5.2 Running balance

`BALANCE` is the account balance after each row, so consecutive rows must satisfy:

```text
previous balance - debit + credit == balance
```

This is the cheapest correctness check the importer will ever get, and it is
required in the preview (FR-032). A break means a misparsed row, a skipped row,
or a row that already exists in the ledger. The preview names the line number
and both balances instead of importing quietly.

### 5.3 Transfers between the owner's own accounts

Some rows are the owner's own money moving between two of their accounts, not
income and not spending. The pattern is a peer-to-peer transfer whose
counterparty is the account holder:

```text
12-Aug-2026,RA AST P2P FT FROM SAMPLE HOLDER HBL ACCT: PK00TEST0000. **103 MSGID: MSG000000000000000001,,0,5000.00,6250.00
```

The real case is a **pass-through account**: money is sent from the primary bank
to this account, then spent from here. Importing these rows as income would
overstate the savings rate by the full amount of every transfer.

They are imported as **transfers**, with the other account as the
counter-account, chosen in the review step and remembered by pattern (FR-033).

### 5.4 Card purchases

Card spending carries a fixed prefix before the merchant:

```text
12-Aug-2026,POS SALE - VISA AT SAMPLE CAFE KARACHI PK,,840.00,0,5410.00
```

The merchant is the text after `AT`, without the trailing city and country. The
importer keeps the full description as the raw value and offers the shortened
merchant for matching, so the remembered merchant → category rule survives
formatting changes (FR-033).

## 6. Synthetic fixture

The shape above, with invented values. This is what the parser tests use.

```csv
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,1250.00
12-Aug-2026,RA AST P2P FT FROM SAMPLE HOLDER HBL ACCT: PK00TEST0000. **103 MSGID: MSG000000000000000001,,0,5000.00,6250.00
12-Aug-2026,POS SALE - VISA AT SAMPLE CAFE KARACHI PK,,840.00,0,5410.00
13-Aug-2026,POS SALE - VISA AT SAMPLE MART LAHORE PK,,1200.50,0,4209.50
13-Aug-2026,ATM CASH WITHDRAWAL SAMPLE BRANCH,,2000.00,0,2209.50
```

Expected result after import into one account with an opening balance:

| Check | Value |
| --- | --- |
| Rows in file | 5 |
| Opening-balance rows | 1 |
| Transactions created | 4 (1 transfer in, 2 expenses, 1 cash withdrawal) |
| Total out | 4,040.50 |
| Total in | 5,000.00 |
| Final balance | 2,209.50 |
| Income | 0 — the transfer is not income |

## 7. How it maps onto the requirements

| Requirement | Effect of this file |
| --- | --- |
| FR-030 | Separate debit/credit columns, `DD-Mon-YYYY` dates, opening row recognised |
| FR-031 | One saved profile, `bank-statement-debit-credit` |
| FR-032 | Preview must include balance continuity and name the first breaking line |
| FR-033 | Review assigns a category **or** a transfer counter-account; both are remembered |
| FR-035 | `INST. NO.` is empty on card rows, so the fallback key (date + amount + description) does the work; a `MSGID:` token is used when present |
| FR-037 | Thousands separators required; quoted commas likely; BOM and preamble unconfirmed |
| FR-038 | The destination account is the pass-through bank account |

## 8. Decisions taken — 2026-11-12

| # | Question | Decision | Consequence |
| --- | --- | --- | --- |
| 1 | Opening-balance row | Imported as the account's opening balance | No zero-amount transaction; later balances still tie out |
| 2 | Balance column | Verified on every row, mismatches flagged in preview | Catches misparsed, skipped and duplicate rows before commit |
| 3 | Own-account transfers | Imported as transfers, not income | Savings rate and spending totals stay truthful |
| 4 | Account structure | Primary bank plus pass-through bank, both tracked | The pass-through statement is the one being imported |

## 9. Still unknown

These do not block the parser, because it must handle them anyway:

- **Delimiters and preamble.** The file was supplied as a spreadsheet image. A
  bank export often has account-title lines above the header, so the parser
  locates the header row instead of assuming line 1.
- **Encoding.** BOM, UTF-8 versus a Windows code page, and non-ASCII merchant
  names are unconfirmed.
- **Whether the primary account's statement will also be imported**, or only its
  opening balance entered by hand. Tracked as Q-35.
- **File size and row count** for the three-month load, which decides whether the
  preview needs paging.
