# Functional requirements

**Status:** proposed — every item below is a candidate for the requirement session, not an agreed decision
**Last updated:** 2026-11-12

Read [the conventions](README.md) and the [glossary](glossary.md) first. Items are grouped by area; IDs are allocated now so discussion can reference them, but `Status` is `Proposed` until explicitly accepted.

Legend: **M**ust / **S**hould / **C**ould / **W**on't (this release).

---

## Accounts

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-001 | M | Create an account with a name, type (asset or liability), currency and opening balance. | Proposed |
| FR-002 | M | Edit an account's name, type and currency; deleting an account requires its balance to be zero or an explicit reassignment of its transactions. | Proposed |
| FR-003 | M | Show each account's current balance and its balance as of any date, derived from the ledger. | Proposed |
| FR-004 | M | Archive an account so it disappears from pickers but keeps its history and still counts in past reports. | Proposed |
| FR-005 | S | Reorder accounts and group them (e.g. Cash, Credit, Loans). | Proposed |
| FR-006 | C | Mark an account as closed with a closing date, excluding it from net worth after that date. | Proposed |

## Transactions

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-010 | M | Record a transaction with date, account, amount, payee and optional category and note. | Proposed |
| FR-011 | M | Amounts are entered and stored exactly; the app never silently rounds a user-entered value. | Proposed |
| FR-012 | M | Edit and delete a transaction, with the effect on balances visible before saving. | Proposed |
| FR-013 | M | Record a transfer between two accounts as a single user action that creates linked postings and is excluded from income and expense reports. | Proposed |
| FR-014 | M | Split a transaction across multiple categories; parts must sum exactly to the total or the save is refused. | Proposed |
| FR-015 | M | Mark a transaction cleared or uncleared, and reconciled separately. | Proposed |
| FR-016 | S | Undo the last destructive action (delete, bulk edit, import) within the session. | Proposed |
| FR-017 | S | Attach a receipt file to a transaction; attachments survive backup and export. | Proposed |
| FR-018 | S | Enter transactions keyboard-first: tab order, date shortcuts, amount arithmetic (`12.50+3`), no mouse required. | Proposed |
| FR-019 | C | Recurring transactions that can be posted manually or on a schedule. | Proposed |
| FR-020 | C | Search transactions by payee, note, amount range, date range, category and account, with results under 100 ms on 20k rows. | Proposed |

## Import

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-030 | M | Import a CSV file with a user-mapped column layout, remembering the mapping per bank profile. | Proposed |
| FR-031 | M | Import OFX/QFX and QIF files without manual column mapping. | Proposed |
| FR-032 | M | Detect likely duplicates against existing transactions and against earlier rows in the same file, and default to skipping them. | Proposed |
| FR-033 | M | Preview an import — counts, date range, total, duplicates, unmatched rows — and commit only on confirmation. | Proposed |
| FR-034 | M | Import is atomic: either the whole confirmed set is written or nothing is. | Proposed |
| FR-035 | S | Apply category rules during import, marking auto-categorised rows so they can be reviewed. | Proposed |
| FR-036 | S | Handle sign conventions, debit/credit columns, thousands separators and multiple date formats per profile. | Proposed |
| FR-037 | C | Import from a folder of statements, one file per account, in a single run. | Proposed |

## Categories and rules

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-040 | M | Create, rename, re-parent and merge hierarchical categories; merging rewrites existing assignments. | Proposed |
| FR-041 | M | Assign exactly one category to a transaction, or one per split part. | Proposed |
| FR-042 | S | Define rules (payee contains, amount range, account) that propose a category, with precedence and a dry-run preview. | Proposed |
| FR-043 | S | Bulk re-categorise a selection and offer to apply the same change to similar transactions. | Proposed |
| FR-044 | C | Archive a category without deleting its history. | Proposed |

## Budgets

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-050 | M | Set a monthly budget amount per category, for a chosen period. | Proposed |
| FR-051 | M | Show budgeted, actual and remaining for the current period, updated as transactions are entered. | Proposed |
| FR-052 | S | Roll over unspent or overspent amounts into the next period, per category, with rollover visible as its own line. | Proposed |
| FR-053 | S | Copy a budget from a previous period, adjusting by a percentage or fixed amount. | Proposed |
| FR-054 | C | Budget by payee or by account as well as by category. | Proposed |

## Reports

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-060 | M | Net worth over time, with a selectable period and an explicit list of included accounts. | Proposed |
| FR-061 | M | Cash flow for a period: income, expenses and net, excluding transfers between own accounts. | Proposed |
| FR-062 | M | Spending by category for a period, drillable to the transactions behind a number. | Proposed |
| FR-063 | M | Every report number can be traced to the transactions that produce it. | Proposed |
| FR-064 | S | Budget variance report: budgeted vs actual per category, with the largest deviations first. | Proposed |
| FR-065 | S | Export any report to CSV, and copy a chart as an image. | Proposed |
| FR-066 | C | Saved report definitions with custom date ranges. | Proposed |

## Reconciliation

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-070 | M | Enter a statement end date and balance, and see the difference against the ledger. | Proposed |
| FR-071 | M | Mark a set of transactions as reconciled in one action; reconciled transactions are locked from editing without an explicit unlock. | Proposed |
| FR-072 | S | Suggest candidate transactions that explain a non-zero difference. | Proposed |
| FR-073 | C | Keep a reconciliation history per account. | Proposed |

## Data, backup and portability

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-080 | M | Back up the entire dataset to a single file, and restore from it, with a version check that refuses incompatible files instead of guessing. | Proposed |
| FR-081 | M | Export all data to a documented, human-readable format (CSV per table plus a manifest) that does not require this app to read. | Proposed |
| FR-082 | M | Apply schema migrations automatically on open, transactionally, with a pre-migration backup. | Proposed |
| FR-083 | S | Scheduled local backups with retention (e.g. keep 10), stored outside the app's working directory. | Proposed |
| FR-084 | S | Verify database integrity on demand and report problems without modifying data. | Proposed |
| FR-085 | C | Import a previously exported dataset to restore or merge. | Proposed |

## Application shell

| ID | Pri | Requirement | Status |
| --- | --- | --- | --- |
| FR-090 | M | Open the last-used dataset on launch, or an empty state that offers to create one. | Proposed |
| FR-091 | M | Command palette / global search reachable from any screen by keyboard. | Proposed |
| FR-092 | S | Undo/redo for the current screen's edits, bounded in memory. | Proposed |
| FR-093 | S | Settings for base currency, date format, number format, first day of week and theme, stored with the dataset. | Proposed |
| FR-094 | C | Multiple datasets (e.g. personal and household) switchable from the menu. | Proposed |
