# ADR-0006 — Deterministic insights, with a seam for a conversational assistant

**Status:** accepted (2026-11-12)
**Date:** 2026-11-12
**Deciders:** project owner
**Supersedes:** —

## Context

The product's third job is to tell the user what to change in order to spend less (brief §1). Two ways to deliver that:

- **Deterministic rules** over the user's own data — offline, explainable, repeatable, and testable.
- **A language model** — flexible phrasing, but needs network and a key, sends financial data off the machine, costs money per call, and can hallucinate a number.

The user chose: insights now, chat later (Q-19). The risk is building insights as ad-hoc UI text that a future assistant cannot reuse.

## Decision

Insights are produced by a **deterministic engine in the domain layer**. Each insight is a value, not a string:

```text
Insight {
  kind,              // CategorySpike, BudgetOverspend, Outlier, MonthSummary
  subject,           // category or merchant id
  observed,          // Money
  baseline,          // Option<Money> — comparison value
  period,            // month
  severity,
  action,            // structured suggestion, not prose
}
```

Rules:

1. **The engine is pure.** It takes a ledger projection and a set of thresholds, and returns insights. No I/O, no clock, no randomness.
2. **Every insight carries its numbers.** The UI formats `observed`/`baseline`; it never re-derives them. If a finding cannot show its arithmetic, it is not shown.
3. **Thresholds are named constants with defaults**, configurable in settings later (spike ≥ 25%, near-limit ≥ 90%).
4. **Insufficient data produces no finding.** The engine returns a `MissingData` reason (e.g. "needs 3 months"), and the UI states it.
5. **Findings are deterministic.** Same ledger and thresholds → same insights in the same order, so they can be snapshot-tested.
6. **A future assistant consumes `Insight` values**, not raw transactions, and may only phrase them. It cannot invent a number: any figure it prints must come from an insight field.

## Consequences

**Positive** — testable with golden fixtures; no network; no data leaves the machine; the same engine powers the insights screen and the month-end summary; adding chat later is a presentation layer, not a rewrite.

**Negative** — the rules will feel mechanical until tuned; thresholds need real data to calibrate; a model could produce subtler observations we will not get.

**Neutral** — the UI owns phrasing and tone; the domain owns the maths.

## Follow-ups

- [ ] Write the rule set as a table in `docs/requirements/` with the threshold for each.
- [ ] Golden tests: a fixed ledger produces a fixed insight list.
- [ ] Decide how the month-end summary is composed from insights (template vs. selection).
- [ ] When chat arrives, keep the same rule: figures only from `Insight`.
