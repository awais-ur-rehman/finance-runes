# Open questions

**Status:** open — these are the agenda for requirement gathering
**Last updated:** 2026-11-12

Each question states why it matters, the options, and a recommendation. When answered, move the answer into the relevant requirement or ADR and mark the question `Answered` with a link. Never delete a question — the reasoning is the value.

**Sessions 1–5 (2026-11-12) closed Q-01, Q-03, Q-04, Q-13 and Q-17…Q-28.** See [Answered](#answered) at the bottom for the decisions and where they were recorded. Q-29…Q-34 were raised during the sessions and are still open.

---

## Blocking architecture

### Q-01 — Double-entry ledger, or single-entry with derived balances?

**Why it matters:** Determines the core data model, every report query, and how transfers and splits are stored. Changing it later means rewriting the domain.

- **A. Double-entry** — every transaction has balanced postings across accounts, with virtual accounts for income and expense. Correct by construction; reports are queries over postings; more concepts to explain in the UI.
- **B. Single-entry** — one row per transaction with an account and a signed amount; transfers are two linked rows. Simpler UI and storage; correctness depends on application logic; categories are labels rather than accounts.

**Recommendation:** A, hidden behind the UI. The UI never says "debit" — but transfers, splits and reconciliation all become trivial and provably correct. Decide in ADR-0005.

### Q-02 — Storage engine?

**Why it matters:** Sets the migration story, backup format, concurrency model and performance ceiling.

- **A. SQLite** (bundled) — mature, single file, transactional, queryable, easy backup via `VACUUM INTO`, well-known to contributors.
- **B. Embedded KV** (redb / sled / fjall) — pure Rust, simple API, but we hand-roll queries and reporting.
- **C. Append-only log + in-memory index** — perfect audit trail and trivial durability, but compaction and querying are ours to build.

**Recommendation:** A. Boring wins for money. Keep it behind a repository trait so the choice stays reversible.

### Q-03 — Multi-currency model?

**Why it matters:** Whether an account's currency or the base currency is the unit of record changes every balance and report.

- **A. Per-account currency, convert at report time** — store amounts in the account's currency; store FX rates separately; revalue reports using rates on the report date.
- **B. Base-currency postings** — convert on entry and store the base amount, keeping the original as metadata.
- **C. Single currency only in v1**, with the schema allowing more later.

**Recommendation:** A, with C as a fallback if the session shows only one currency is used. Never B — it destroys history when rates change.

### Q-04 — Is encryption at rest in v1?

**Why it matters:** Adds key management, a recovery story and a performance cost; omitting it is a trust decision that must be documented.

**Options:** rely on OS disk encryption (FileVault/BitLocker) and document it; SQLCipher-style page encryption with a passphrase; encrypted backup only.

**Recommendation:** OS-level only in v1, stated plainly in the README and onboarding. Revisit if the dataset ever leaves the machine.

---

## Product behaviour

### Q-05 — How opinionated should categorisation be?

Auto-categorisation from rules, from payee history, or nothing until asked? **Recommendation:** rules first (predictable, inspectable), learned suggestions later, always shown as suggestions with a one-key accept.

### Q-06 — Budget model: category budgets or envelope budgeting?

Category budgets are simpler; envelopes (YNAB-style) change how users think about money and require handling overspend explicitly. **Recommendation:** category budgets with rollover in v1; envelopes are a different product.

### Q-07 — Should reconciled transactions be editable?

Locking prevents silent history rewrites but frustrates users who mistyped a date. **Recommendation:** lock, with an explicit unlock that records the change.

### Q-08 — What happens to a deleted transaction?

Hard delete, soft delete with an audit trail, or a reversing entry? **Recommendation:** soft delete with an audit record, hidden from the UI, purged only on export.

### Q-09 — Import deduplication strictness?

Exact fingerprint match only, or fuzzy matching on date ±1 day and amount? Fuzzy catches re-posts but risks merging two genuine identical purchases. **Recommendation:** exact by default, fuzzy surfaced as "possible duplicate" and never auto-skipped.

### Q-10 — Where do attachments live?

Inside the database (blobs) or beside it in a managed folder? **Recommendation:** beside it, referenced by content hash, included in backup and export.

### Q-11 — Multiple datasets in v1?

Separate files (personal, household) or one dataset with tags? **Recommendation:** multiple dataset files; it is the same code path as "open a file".

### Q-12 — What does "today" mean for a transaction dated in the future?

Allowed and excluded from balances until the date arrives, or refused? **Recommendation:** allowed, shown as scheduled, excluded from "current" balances.

---

## Process

### Q-13 — Which banks/formats must the importers support first?

**Status:** answered 2026-11-12 — one CSV shape, see [the import profile](csv-import-profile.md). OFX/QFX/QIF are deferred (FR-039).

### Q-14 — Which platform is the primary target?

Development appears to be on Windows. Confirm whether macOS is equally important for the first release.

### Q-15 — Is the app for one person only, forever?

If a second user (partner, household) is plausible, some schema decisions (ownership, per-user settings) change now rather than later.

### Q-16 — What is the acceptable install/packaging story?

Signed installer, portable zip, `cargo install`? Affects CI and release work more than the app.

**Answered 2026-11-12:** a portable archive per platform, no installer, no signing in v1 (Q-27).

---

## Raised in sessions 3–5

### Q-29 — What columns does your bank CSV actually have?

**Status:** answered 2026-11-12 — see [the import profile](csv-import-profile.md).

**Answer:** six columns — `DATE`, `PARTICULARS`, `INST. NO.`, `DEBIT`, `CREDIT`, `BALANCE`. Dates are `DD-Mon-YYYY`, debit and credit are separate columns with thousands separators, every row carries a running balance, and the first row is an opening-balance marker rather than a transaction.

Two consequences the requirements did not anticipate: the running balance enables a continuity check before commit, and some rows are the owner's own money moving between two of their accounts, which must import as transfers rather than income.

### Q-30 — Where does the savings account come from?

Budget surplus is offered as a transfer to a savings account (FR-053), but session 2 chose only bank and cash.

**Options:** (a) first run offers to create a `savings` account, skippable; (b) the destination is any account the user labels; (c) surplus is tracked only, with no transfer.

**Recommendation:** (a). It keeps the concept explicit without forcing a second account on someone who does not want one.

### Q-31 — Is month close explicit or automatic?

The surplus-to-savings action moves real money, so it needs a moment where the user agrees the month is done.

**Options:** explicit "close month"; automatic on the first of the next month; auto-suggest with confirmation.

**Recommendation:** explicit close (FR-093), because it gates a money-moving suggestion.

### Q-32 — What is "unspent budget" when only some categories have budgets?

Only categories with a limit contribute. The edge cases are what matter: a category with no budget, an overspent category, and a month with no budgets at all.

**Recommendation:** sum `max(0, budget − spent)` over categories that have a budget; subtract overspends; floor the offer at zero; show the per-category breakdown behind the number.

### Q-33 — Should recurring items post automatically?

**Recommendation:** confirm each one in v1 (FR-058). Auto-posting (FR-060) is a later convenience, not a v1 risk worth taking.

### Q-34 — Is deleting a transaction a soft delete?

**Recommendation:** soft delete with an audit record hidden from the UI, purged only on full export. It makes undo honest and costs one column.

### Q-35 — Is the primary bank account imported too, or only entered by hand?

The imported statement belongs to the **pass-through** account. Money reaches it
from the primary bank, so the other side of every transfer is an account whose
history we do not have.

**Options:** (a) track the primary account with an opening balance only, and let
transfers adjust it from there; (b) import the primary account's statement as
well, giving both sides real history; (c) do not track the primary account, and
record the transfers as a non-income category instead.

**Recommendation:** (a) for the first import. It keeps the ledger honest with one
statement, and (b) stays open if the second statement turns out to be easy to
export.

---

## Answered

| Question | Answer | Recorded in |
| --- | --- | --- |
| Q-01 — ledger model | Hidden double-entry; UI shows no accounting terms | [ADR-0005](../architecture/decisions/0005-ledger-model.md) |
| Q-03 — multi-currency | PKR only in v1; `Money` still carries a currency code | [Brief §4](../00-project-brief.md#4-scope) |
| Q-04 — encryption at rest | OS disk encryption only, stated plainly in the README and onboarding | [Brief §5](../00-project-brief.md#5-decisions-so-far) |
| Q-13 — bank formats | One CSV shape, now known: six columns with separate debit and credit; OFX/QFX/QIF stay deferred | [Import profile](csv-import-profile.md) |
| Q-17 — income | Record income as well as expenses | [Brief §5](../00-project-brief.md#5-decisions-so-far) |
| Q-18 — budgets | Monthly limit per category, with overspend flags | [Brief §5](../00-project-brief.md#5-decisions-so-far) |
| Q-19 — advice mechanism | Rule-based insights in v1; conversational assistant later, behind a seam | [Brief §5](../00-project-brief.md#5-decisions-so-far) |
| Q-20 — payment methods | Bank account and cash; wallet and card modelled but no UI | [Brief §5](../00-project-brief.md#5-decisions-so-far) |
| Q-21 — "where" | Merchant/payee name plus a free-text note | [Brief §5](../00-project-brief.md#5-decisions-so-far) |
| Q-22 — entry fields | Amount, category, merchant, account; date defaults to today | [Brief §5](../00-project-brief.md#5-decisions-so-far) |
| Q-23 — budget meaning | A cap; unspent is offered as a confirmed transfer to savings, never automatic | [FR-053](functional-requirements.md#budgets) |
| Q-24 — starting history | One-time CSV import of roughly three months | [FR-030…038](functional-requirements.md#csv-import--one-time-historical-load) |
| Q-25 — backup | Automatic on exit when data changed, keeping 10, plus manual | [FR-080](functional-requirements.md#data-backup-and-portability) |
| Q-26 — number format | South Asian grouping by default, configurable | [NFR-050](non-functional-requirements.md#formatting-and-internationalisation) |
| Q-27 — packaging | Portable archive per platform; no installer or signing in v1 | [NFR-031](non-functional-requirements.md#portability-and-durability) |
| Q-28 — language | English only, but every string externalised for later translation | [NFR-055](non-functional-requirements.md#formatting-and-internationalisation) |
| Q-29 — bank CSV columns | Six columns, `DD-Mon-YYYY`, separate debit and credit, a running balance on every row, opening-balance marker first | [Import profile](csv-import-profile.md) |

Also resolved during the sessions, without a dedicated row above: Q-05 (no rules engine; the app remembers merchant → category), Q-06 (category caps, not envelopes), Q-09 (dedupe by reference, else date + amount + description), Q-11 (multiple datasets deferred), Q-14 (all three platforms equally), Q-15 (single user in v1).

Still genuinely open: Q-12 (future-dated transactions), Q-02 (storage sign-off), and Q-29…Q-34.
