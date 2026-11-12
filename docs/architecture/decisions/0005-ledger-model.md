# ADR-0005 — Double-entry ledger behind a single-entry UI

**Status:** proposed
**Date:** 2026-11-12
**Deciders:** project owner
**Supersedes:** —

## Context

Two ways to model money movement:

- **Single-entry:** one record per transaction, with an account and a signed amount. Transfers are two linked records.
- **Double-entry:** every transaction is a set of postings that sum to zero, with virtual accounts representing income and expense.

Personal finance apps usually choose single-entry because it is easier to explain. The cost appears later: transfers that are counted as both income and expense, splits that do not add up, categories that silently disagree with balances, and reconciliation that cannot explain a difference.

## Decision

Use a **double-entry ledger** internally, presented as a single-entry register.

Every transaction has two or more postings whose amounts sum to zero. Income and expense are virtual accounts, invisible in the account list. The UI shows "−120.00 · Groceries" and never the word "debit".

Invariants enforced in `finance-core`:

1. Postings sum to zero within a transaction, in one currency.
2. A transfer is exactly two postings on two different accounts, and is excluded from income/expense reports by construction (it touches no virtual account).
3. A split assigns a category per posting; the parts sum to the total.
4. Posting currency equals account currency.
5. Balances are derived from postings; no stored balance is authoritative.

## Consequences

**Positive** — transfers cannot distort reports; every report is a query over postings; reconciliation differences are explainable; splits are provably exact; audit trail is inherent.

**Negative** — more concepts in the domain and the schema; imports must map bank rows to balanced postings; contributors need the glossary. UI copy must hide the accounting vocabulary, which takes deliberate design.

**Neutral** — reports become slightly more abstract (filter by account kind rather than by a flag).

## Follow-ups

- [ ] Define virtual accounts (income, expense, opening balance, FX gain/loss).
- [ ] Property test: for any generated ledger, total postings per currency sum to zero and net worth equals assets minus liabilities.
- [ ] Decide how a single-entry import maps to postings without inventing categories.
