# Open questions

**Status:** open — these are the agenda for requirement gathering
**Last updated:** 2026-11-12

Each question states why it matters, the options, and a recommendation. When answered, move the answer into the relevant requirement or ADR and mark the question `Answered` with a link. Never delete a question — the reasoning is the value.

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

Need real (anonymised) samples for each. Without a fixture corpus, import quality is guesswork. **Needs input from the user.**

### Q-14 — Which platform is the primary target?

Development appears to be on Windows. Confirm whether macOS is equally important for the first release.

### Q-15 — Is the app for one person only, forever?

If a second user (partner, household) is plausible, some schema decisions (ownership, per-user settings) change now rather than later.

### Q-16 — What is the acceptable install/packaging story?

Signed installer, portable zip, `cargo install`? Affects CI and release work more than the app.

---

## Answered

_None yet._
