# Glossary

Domain language for this project. Use these words exactly, in code, docs, UI copy and commit messages. If a term is missing, add it here in the same PR that introduces it.

| Term | Meaning | Not to be confused with |
| --- | --- | --- |
| **Account** | A container for money you own or owe: checking, savings, credit card, cash, loan. Has one currency. | A bank login or a user profile. |
| **Asset account** | An account with a positive balance meaning you have money (checking, savings, cash). | |
| **Liability account** | An account where a positive balance means you owe money (credit card, loan). Displayed as negative net worth contribution. | |
| **Transaction** | A dated event that moves money in or out of exactly one account, or between two. | A transfer (which is two linked transactions). |
| **Posting** | The effect of a transaction on one account: an amount and a direction. A transfer has two postings. | A transaction — one transaction may have several postings. |
| **Ledger** | The append-only record of all transactions. The single source of truth. | A report, a view, or a balance. |
| **Balance** | The sum of postings for an account up to a point in time. Always *derived*, never stored as truth. | A "current balance" field typed in by a user. |
| **Opening balance** | The balance an account had before the first transaction recorded in this app. Stored as a special transaction, not as an account field. | |
| **Transfer** | Money moving between two of your own accounts. Not income, not expense. | A payment to someone else (that is an expense). |
| **Category** | A label describing *why* money moved: Groceries, Rent, Salary. Hierarchical (parent → child). | An account — categories never hold money. |
| **Split** | One transaction divided across several categories. Parts sum exactly to the total. | Multiple transactions. |
| **Payee** | Who the money went to or came from. Free text, deduplicated by the app. | A category. |
| **Reconciliation** | Comparing the ledger against a bank statement and confirming they agree. | Importing (which only adds transactions). |
| **Cleared / Uncleared** | Whether a transaction has appeared on a statement yet. | Reconciled — a stronger state. |
| **Budget** | A planned spending limit for a category over a period. | A forecast of actual balance. |
| **Rollover** | Carrying an unused or overspent budget remainder into the next period. | Compounding interest. |
| **Base currency** | The currency used to total accounts and produce reports. | An account's own currency. |
| **Minor unit** | The smallest indivisible unit of a currency: cents for USD/EUR, yen for JPY, fils for KWD. Amounts are stored as integers in minor units. | A "cent" — not every currency has 2 decimal places. |
| **Currency code** | ISO 4217 alphabetic code, e.g. `USD`, `EUR`, `JPY`. | A currency symbol, which is ambiguous. |
| **FX rate** | The conversion rate between two currencies on a date. | A price. |
| **Import** | Reading transactions from an external file (CSV, OFX, QIF) into the ledger. | Sync — there is no sync. |
| **Fingerprint** | A stable hash of the fields that identify a transaction, used to detect duplicates on import. | A hash of the whole record. |
| **Statement balance** | The balance the bank reports at the end of a period. The target for reconciliation. | The ledger balance. |
| **Net worth** | Sum of asset balances minus liability balances, in the base currency. | Cash flow, which is movement over a period. |
| **Cash flow** | Money in minus money out over a period, excluding transfers between own accounts. | Net worth change (which includes revaluation). |
