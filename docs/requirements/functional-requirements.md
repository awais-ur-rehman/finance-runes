# Functional requirements

**Status:** accepted for v1 — sessions 1–5 complete, 2026-11-12
**Last updated:** 2026-11-12

Derived from five requirement sessions. Read [the conventions](README.md) and the [glossary](glossary.md) first.

**Legend:** **M**ust · **S**hould · **C**ould · **W**on't (this release). Everything marked M/S/C is in scope for v1. `W` items keep their IDs for traceability.

---

## Accounts

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-001 | M | Create an account with a name, kind (bank, cash, savings, card, wallet), currency and opening balance. Only bank and cash need UI in v1. |
| FR-002 | M | Edit an account's name and kind. Deleting is allowed only when it has no transactions; otherwise the account can be archived. |
| FR-003 | M | Show each account's current balance and its balance as of any date, derived from the ledger. |
| FR-004 | M | Archive an account so it disappears from pickers while its history remains in past months. |
| FR-005 | M | Record a transfer between two accounts as one action, creating linked postings. Transfers are excluded from income and expense totals. |
| FR-006 | S | Designate one account as the savings destination used by the month-end surplus action (FR-053). |
| FR-007 | C | Support more than one account of the same kind. |

## Transactions

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-010 | M | Record an expense with amount, date (default today), category, merchant, account and an optional note. |
| FR-011 | M | Record income with amount, date, source, income category, account and an optional note. |
| FR-012 | M | Amounts are entered and stored exactly; the app never silently rounds a user-entered value. |
| FR-013 | M | Edit any transaction; balances and reports reflect the change immediately. |
| FR-014 | M | Delete a transaction after a single confirmation, with an immediate undo available. The row is kept with a deletion marker and excluded from every view, so undo restores it exactly. |
| FR-015 | M | Warn — without blocking — when a new transaction matches an existing one on amount, merchant and date. |
| FR-016 | M | Direction is explicit: an amount is always positive and the transaction is either income or expense. Negative amounts are rejected. |
| FR-017 | S | Keyboard-first entry: `Enter` saves, `Tab` moves forward, date accepts shortcuts (`today`, `-1d`), amount accepts simple arithmetic (`450+80`). |
| FR-018 | S | Open the entry form from any screen with one shortcut. |
| FR-019 | W | Split one expense across categories. Deferred: the model allows it, the UI does not. |
| FR-020 | M | Search transactions by merchant text, category, account, date range and amount range, with a running total for the result set. |

## Categories

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-040 | M | Two-level categories: a parent with children (Food → Groceries, Restaurants). A parent may also be used directly. |
| FR-041 | M | Create, rename, re-parent and delete categories. Deleting a category that has transactions requires choosing a replacement. |
| FR-042 | M | Merge two categories, rewriting existing assignments. |
| FR-043 | M | Seed a starter set on first run — Food & Dining, Groceries, Transport, Bills, Rent, Health, Education, Shopping, Entertainment, Family, Other — all editable. |
| FR-044 | M | Income and expense categories are separate lists; an income category cannot be assigned to an expense. |
| FR-045 | S | Reorder categories and hide ones no longer used. |

## CSV import — one-time historical load

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-030 | M | Import a CSV with a user-mapped layout: date column, description column, and either one signed amount column or separate debit/credit columns. A row that carries only the opening balance is recognised and imported as the account's opening balance, not as a transaction. |
| FR-031 | M | Save the mapping as a named bank profile and reuse it on the next import. |
| FR-032 | M | Preview before committing: row count, date range, total in, total out, detected duplicates, unparseable rows with their line numbers, and any break in the running balance the file provides, naming the first offending line and both balances. |
| FR-033 | M | Review screen: assign a category to each row, or mark it as a transfer to another of your own accounts, before saving. The app remembers merchant → category and pattern → transfer, and applies both to later rows and future entries. |
| FR-034 | M | Commit is atomic: either every confirmed row is written or nothing is. |
| FR-035 | M | Detect duplicates against existing transactions and within the file — by reference number when present, including a bank message id embedded in the description, otherwise by date, amount and description. Default to skipping; the user can override. |
| FR-036 | M | Imported transactions are marked with their source and remain editable like any other transaction. |
| FR-037 | S | Handle BOM, CRLF, quoted commas, thousands separators, multiple date formats and parenthesised negatives. |
| FR-038 | S | Choose the destination account at import time. |
| FR-039 | C | Import OFX/QFX/QIF. |

## Budgets

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-050 | M | Set a monthly budget amount per expense category. |
| FR-051 | M | Show budgeted, spent and remaining for the current month, per category and in total. |
| FR-052 | M | Flag a category as over budget when spending exceeds its limit, and as near-limit at a configurable threshold (default 90%). |
| FR-053 | M | At month end, offer to transfer the unspent budget total to the designated savings account. The transfer is shown for confirmation and, once accepted, is recorded as a real transfer transaction. |
| FR-054 | M | A budget applies to every month until changed. Budget changes are dated, so past months keep the limit that was in force. |
| FR-055 | S | Copy the previous month's budget into the current one, optionally adjusting by a percentage. |
| FR-056 | W | Roll unspent budget into next month's spending limit. Not chosen: unspent money is offered as savings instead (FR-053). |

## Recurring expenses

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-057 | M | Mark a transaction as recurring with a frequency (monthly, weekly, yearly) and a next-due date. |
| FR-058 | M | Show due and upcoming recurring items, and create the transaction on confirmation with one action. |
| FR-059 | S | Show total committed recurring spend for the current month. |
| FR-060 | C | Post recurring items automatically without confirmation. |

## Savings goal

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-061 | M | Set a monthly savings goal amount. |
| FR-062 | M | Show progress against the goal, where saved = income − expenses for the month. Transfers between own accounts do not affect it. |
| FR-063 | S | Show goal history: months met, months missed, average saved. |

## Month view and reports

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-064 | M | Month view shows, without scrolling: total spent with change vs the previous month, the savings rate, and the category breakdown. |
| FR-065 | M | Month selector, defaulting to the current month. |
| FR-066 | M | Every figure can be drilled into and traced to the transactions that produce it. |
| FR-067 | S | Merchant breakdown for the month. |
| FR-068 | S | Trend chart across the last 3, 6 or 12 months. |
| FR-069 | C | Export a report to CSV, and copy a chart as an image. |

## Savings insights

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-100 | M | An Insights screen lists current findings. Each states the amount, the category or merchant, the comparison period, and a suggested action. |
| FR-101 | M | Category spike: a category more than a threshold (default 25%) above its 3-month average, with both numbers shown. |
| FR-102 | M | Budget warnings: over budget, or within the near-limit threshold with days left in the month. |
| FR-103 | M | Outliers: the largest transactions of the month and any transaction far above the merchant's usual amount. |
| FR-104 | M | Month-end summary: a short, generated summary of what changed and what to adjust, available for past months. |
| FR-105 | M | Insights are deterministic and explainable: the same data yields the same findings, and each finding shows the numbers behind it. |
| FR-106 | M | No finding is produced without sufficient data; the screen says what is still missing instead of inventing a pattern. |
| FR-107 | S | Dismiss a finding for the current period; it does not reappear until the data changes. |
| FR-108 | C | Conversational assistant. Deferred, but the insights layer exposes a seam so it can be added without a rewrite. |

## Data, backup and portability

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-080 | M | Back up automatically on exit when data changed, keeping the last 10, plus a manual "back up now" action. |
| FR-081 | M | A backup is a single consistent file, stamped with schema version and creation time. |
| FR-082 | M | Restore validates version and integrity before replacing anything, and refuses incompatible files with a clear message. |
| FR-083 | M | Export all data to documented CSV files plus a manifest, readable without this app. |
| FR-084 | M | Migrations run automatically on open, transactionally, after taking a pre-migration backup. |
| FR-085 | S | Run an integrity check on demand and report problems without modifying data. |
| FR-086 | S | Let the user choose the backup directory. |

## Application shell and settings

| ID | Pri | Requirement |
| --- | --- | --- |
| FR-090 | M | First run: create a dataset, set the name, seed the starter categories, optionally create a savings account. |
| FR-091 | M | Reopen the last-used dataset on launch. |
| FR-092 | M | Settings for number format (default South Asian), date format, week start, theme, savings goal and backup location. |
| FR-093 | S | Close a month automatically on the first day of the next month, computing its surplus (FR-053) and offering it as a transfer. A closed month stays editable: a late entry is recorded normally and the offer recalculates until it is accepted, after which the month is settled and no second offer is made. |
| FR-094 | S | Command palette reachable from any screen. |
| FR-095 | C | Multiple datasets (personal, household) switchable from the menu. |

## Deferred — keep IDs, do not build in v1

| ID | Requirement | Why |
| --- | --- | --- |
| FR-070…073 | Reconciliation against statement balances | Manual entry plus a one-time import makes it unnecessary in v1 |
| FR-019 | Splitting a transaction across categories | Entry stays four fields |
| FR-056 | Budget rollover | Unspent budget is offered as savings instead |

---

## Detailed acceptance criteria

The requirements most likely to be implemented wrongly. Everything here is testable.

### FR-010 — record an expense

- Given the entry form is open, when I type `450`, choose `Groceries`, type `Imtiaz`, and press `Enter`, then one expense of PKR 450 is saved against today, Groceries and Imtiaz.
- Given I have not chosen an account, when I save, then the default account is used and the row is saved without extra prompting.
- Given I enter `0` or leave the amount empty, when I save, then nothing is saved and the amount field is focused with an explanation.

### FR-014 — delete with undo

- Given I delete a transaction, then it disappears from the register, balances, month totals, budgets and insights immediately.
- Given I press undo, then the transaction returns with the same identifier, date, amount, account, category, payee and note, and every total is back to its previous value.
- Given the undo window has passed, then the row is still kept with a deletion marker, hidden from every view, and removed only by a full export (Q-34).

### FR-015 — duplicate warning

- Given an expense of 450 at Imtiaz dated today exists, when I enter the same amount, merchant and date, then a non-blocking warning appears and the save still succeeds if I confirm.
- Given the same amount at the same merchant on a different date, then no warning appears.

### FR-033 / FR-035 — import review and duplicates

- Given a CSV with 120 rows, 8 of which already exist, when I open the preview, then it reports 120 parsed, 8 duplicates, 112 new, and the total in and out for the new rows only.
- Given a row whose date cannot be parsed, then it is listed with its line number and excluded from the commit, and the remaining rows can still be imported.
- Given I assign `Groceries` to `IMTIAZ STORE`, when a later row has the same description, then `Groceries` is preselected for it.
- Given the commit fails partway, then no row is written.
- Given the first row is an opening-balance marker, when I open the preview, then it is reported as the opening balance, not as a transaction, and it appears in neither the in total nor the out total.
- Given a row's stated balance disagrees with the previous balance plus its amount, then the preview names that line and both balances, and still allows the remaining rows to be imported.
- Given a row is marked as a transfer to another of my accounts, then it is excluded from income and expenses, and a later row with the same pattern is preselected as that same transfer.

### FR-053 — unspent budget to savings

- Given budgets of 20,000 (Groceries) and 10,000 (Transport), with spending of 18,000 and 9,500 in a closed month, when I open the month-end action, then it offers a transfer of PKR 2,500 to the designated savings account.
- Given a category is over budget, then its overspend reduces the offered total and is shown as a separate line; the offer never goes below zero.
- Given I accept, then one transfer transaction is created and the month's budget status is marked as settled.
- Given no savings account is designated, then the action explains how to designate one instead of failing silently.

### FR-093 — automatic month close

- Given the first day of a month arrives, then the previous month is closed without an action from me.
- Given a closed month, when I add, edit or delete a transaction in it, then the change is accepted and the month's totals and surplus recalculate.
- Given I accepted the surplus transfer for a month, then that month is settled and no second offer is made for it, even if a later correction changes its surplus.
- Given a month with no budgets, then it closes with no offer and no error.

### FR-062 — savings rate

- Given income 150,000 and expenses 120,000 in a month, then the savings rate is 20%.
- Given a transfer of 30,000 from bank to savings, then the savings rate is unchanged.
- Given income of 0 and expenses above 0, then the rate is shown as not applicable rather than as a division by zero.

### FR-101 — category spike

- Given Groceries averaged 12,000 over the previous three months and is 16,500 this month, then a finding appears stating both numbers and the increase of 37.5%.
- Given only two months of history, then no spike finding is produced and the screen says three months are needed.

### FR-080 / FR-082 — backup and restore

- Given I close the app after adding a transaction, then a backup is written and the 11th oldest is deleted.
- Given I restore a backup from a newer schema version, then the restore is refused, the current dataset is untouched, and the message names both versions.
- Given I restore a corrupt file, then integrity validation fails before any write.

---

## Traceability

Every accepted requirement above must be referenced by at least one test name or PR before it is marked implemented. Requirements without tests are tracked in the roadmap, not in a "done" column.
