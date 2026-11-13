# ADR-0007 — Remembered rules are dataset state, not UI memory

**Status:** accepted (2026-11-13)
**Date:** 2026-11-13
**Deciders:** project owner
**Supersedes:** —

## Context

FR-033 asks the app to remember two things and apply them later: "merchant →
category" and "pattern → transfer". Until now a rule lived only in the import
review session's memory, so it died with the session: every import asked the same
questions again.

Three ways to make a rule outlive a restart:

- **A settings file beside the dataset** — easy, but a second source of truth
  that a backup, a restore or an export can miss.
- **A storage-only table of opaque keys** — small, but the key format becomes a
  stringly-typed contract the app has to agree with itself about, and a rule
  learned during a commit is written in a second transaction.
- **Domain state, saved with the ledger** — the rule is part of what the user
  taught the app, and one save covers both.

The key is not free-form: a merchant name is derived by the importer, and a
transfer is recognised as one of two patterns. Both are vocabulary the domain
can own; only the *matching* against a particular statement row is importer
knowledge.

## Decision

A rule is **domain state** (`finance_core::Rule`) and lives in the ledger:

```text
Rule { key: RuleKey, target: RuleTarget }

RuleKey    = Merchant(String) | Transfer(TransferPattern)
TransferPattern = CashWithdrawal | PeerTransfer { counterparty: Option<String> }
RuleTarget = Category(CategoryId) | Account(AccountId)
```

Rules:

1. **Keys are canonical at construction.** A merchant key is trimmed and ASCII
   upper-case; a blank name is refused (`RuleKey::merchant` returns `None`). A
   blank counterparty is `None`. Two spellings of one merchant teach one rule.
2. **The key is the identity.** `Ledger::add_rule` replaces a rule with the same
   key, so re-teaching a row changes the answer instead of stacking a second
   opinion behind the first.
3. **The ledger validates the target.** A rule cannot point at an account or
   category the ledger does not have; it is refused, not stored and ignored.
4. **Rules are written with the rows that taught them.** `Ledger::apply` takes
   transactions and rules as one batch and is all-or-nothing (FR-034), so a failed
   commit never leaves a remembered choice about rows that were not written.
5. **The pattern type moves into the domain.** `TransferPattern` replaces the
   importer's `TransferHint`: a cash withdrawal and a peer transfer are facts
   about the user's money, not about CSV. The parser still *recognises* them and
   still calls the result a hint.
6. **Matching stays in the application layer.** The domain cannot see a statement
   row, so `finance-app` decides whether a key matches a row.
7. **The description is never a key.** A description can carry an account number
   or a reference; a rule keyed on one would stop matching the next transfer and
   copy the number into later rows.

## Consequences

**Positive** — rules survive a restart, a backup, a restore and an export with no
second mechanism; an import of a file the app has seen before needs fewer
decisions; the database stores typed columns, not an opaque key string; and the
"save the ledger" path already carries them.

**Negative** — the domain grows a concept that exists to help classification, and
schema v3 is another migration to keep working. The pattern type is now shared by
two crates, so a change to it is a change to both.

**Neutral** — a rule is not history and never changes a recorded transaction; it
only preselects rows in a review the user can still override.

## Follow-ups

- [x] Move `TransferHint` into the domain as `TransferPattern`.
- [x] Schema v3: a `rules` table with a migration test from every earlier version.
- [x] `Ledger::apply` writes transactions and rules atomically.
- [ ] Show the remembered rules in settings, so the user can see and delete them.
- [ ] Delete the rules that point at a category or account when that is removed.
