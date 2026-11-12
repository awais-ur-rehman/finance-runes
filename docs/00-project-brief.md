# Project brief

**Status:** draft — awaiting requirement gathering
**Last updated:** 2026-11-12
**Owner:** _TBD_

## 1. Problem

People who want to understand their own money face a false choice:

| Option | What it costs you |
| --- | --- |
| Cloud budgeting apps | Your full transaction history sits on someone else's server, behind a subscription, and disappears if the company does. |
| Spreadsheets | Total control, but no validation, no importers, fragile formulas, and no answer to "why is this month different?". |
| Desktop accounting software | Built for businesses: double-entry ceremony, jargon, and a UI from 2009. |

The gap: **a personal tool with the rigour of accounting software and the data ownership of a spreadsheet.**

## 2. Target user

_Primary persona — to be validated during requirement gathering._

A single individual who:

- has 2–8 accounts across 1–3 currencies (checking, savings, one or two credit cards, maybe cash),
- wants to know where money went last month without doing manual bookkeeping,
- imports bank statements monthly rather than typing every transaction,
- values privacy and offline access over sync and collaboration,
- is comfortable installing a desktop app, not necessarily a developer.

Secondary persona: someone tracking a side income or freelancing alongside a salary, who needs category-level reporting but not invoicing.

## 3. Value proposition

> Your complete financial history, on your disk, correct to the cent, and fast enough to feel like a native app — because it is one.

Three differentiators, in priority order:

1. **Correct by construction.** Append-only ledger, integer money, derived balances. Reconciliation tells you when reality and the ledger disagree instead of hiding it.
2. **Yours.** Local database, documented format, one-click export, no account, no telemetry.
3. **Pleasant.** Native GPU-rendered UI via Rune: instant search, keyboard-first entry, 120 Hz scrolling, no spinner between you and your data.

## 4. Scope

### In scope for v1

- Accounts (asset and liability), multi-currency balances.
- Transaction register: create, edit, delete, split, transfer, annotate.
- Import from CSV/OFX/QIF with mapping, dedupe and a review step.
- Categories (hierarchical), rules for auto-categorisation.
- Budgets (monthly, per category, with rollover).
- Reports: net worth, cash flow, spending by category, budget variance.
- Reconciliation against statement balances.
- Backup/restore and full export.

### Out of scope for v1

- Multi-user, sharing, sync, cloud accounts.
- Bank API aggregation.
- Investments, securities, market prices.
- Invoicing, payroll, tax filing.
- Mobile apps; web build is a stretch goal, not a commitment.

### Deferred, but designed for

- Recurring/scheduled transactions and forecasting.
- Attachments (receipts) — storage layout must allow it.
- Plugins or scripting.
- Encryption at rest beyond OS-level disk encryption.

## 5. Success criteria

Measurable, and checkable without a marketing department:

| Criterion | Target |
| --- | --- |
| A month of transactions can be imported, reviewed and categorised | under 5 minutes for a 200-row statement |
| Balance correctness | 100% agreement with statement balances after reconciliation, across 500 randomised fixtures |
| Cold start to interactive register | under 1.5 s on a 5-year dataset (≈20k transactions) |
| Scroll and search | no dropped frames while scrolling 20k rows; search results under 100 ms |
| Data durability | crash during write never leaves an unreadable database |
| Portability | export produces a documented format readable without this app |
| Privacy | zero network calls in normal operation, verified by test |

## 6. Constraints and assumptions

- **Rune is pre-release.** Pin a `dev` rev, expect breakage on upgrade, and keep the UI layer thin so a Rune change cannot reach the domain.
- **Single developer, agent-assisted.** Prefer boring, well-documented solutions over clever ones. Every non-obvious decision gets an ADR.
- **No backend.** Anything requiring a server is out of scope by definition.
- **Desktop only.** Windows first (development machine), then macOS and Linux.
- **Offline by default.** The only network access would be explicit and user-initiated.

## 7. Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Rune pre-release churn breaks the UI layer | High | Thin UI crate, pinned rev, headless render tests to catch regressions |
| Importers mis-handle real bank formats | High — wrong money | Fixture corpus per bank, preview before commit, property tests for parsing |
| Money rounding errors | High — trust | Integer minor units, explicit rounding policy, no float anywhere in the domain |
| Scope creep into full accounting | Medium | Non-goals list above; double-entry only if reconciliation requires it |
| Data loss on crash | High | Transactional writes, WAL, backups, restore tested in CI |
| Requirements drift between docs and code | Medium | Requirement IDs in PRs, docs updated in the same PR |

## 8. Open decisions

Tracked in [`requirements/open-questions.md`](requirements/open-questions.md). The ones that block architecture:

- Q1 — double-entry ledger or single-entry with derived balances?
- Q2 — storage engine (SQLite vs. embedded KV vs. append-only log + index)?
- Q3 — multi-currency model: per-account currency with conversion at report time, or base-currency postings?
- Q4 — is encryption at rest in v1?

## 9. Next step

Requirement gathering session. Output: filled [`functional-requirements.md`](requirements/functional-requirements.md) and [`non-functional-requirements.md`](requirements/non-functional-requirements.md), answered open questions, and the first accepted ADRs.
