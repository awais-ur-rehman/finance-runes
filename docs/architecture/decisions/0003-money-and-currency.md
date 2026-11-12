# ADR-0003 — Money as integer minor units plus currency

**Status:** accepted (2026-11-12)
**Date:** 2026-11-12
**Deciders:** project owner
**Supersedes:** —

## Context

Floating point cannot represent most decimal fractions exactly. `0.1 + 0.2 != 0.3` is not a curiosity when the values are money: repeated addition, splitting a transaction, and currency conversion all accumulate error, and a user who sees a balance off by one cent stops trusting the app.

Currencies also differ in precision: USD/EUR have 2 decimal places, JPY has 0, KWD has 3. A model that assumes "cents" is wrong for a third of the world.

## Options considered

| Option | Pros | Cons |
| --- | --- | --- |
| `f64` | Simple, fast, built in | Inexact; rounding errors are invisible until they are not; unsuitable for stored values |
| `rust_decimal` | Exact decimal, ergonomic arithmetic, handles division | Larger, slower, and still needs an explicit rounding policy; can be used carelessly |
| **Integer minor units + currency** | Exact by construction, compact, fast, serialises trivially, matches how banks store amounts | Every operation must be currency-aware; display formatting needs a currency table |

## Decision

Represent money as:

```text
Money { minor: i64, currency: Currency }
```

where `minor` is the amount in the currency's smallest unit (cents for USD, yen for JPY, fils for KWD), and `Currency` is an ISO 4217 code carrying its own exponent.

Rules:

1. **No `f64` in the domain.** The type system should make it awkward to introduce one; `Money` has no `f64` conversions.
2. **Arithmetic is checked.** Add/subtract require matching currencies and return a typed error otherwise. Multiply/divide by a scalar takes an explicit rounding mode.
3. **Parsing and formatting live at the edges.** User input is parsed to `Money` once; display formats from `Money`. Storage stores integers.
4. **Conversion is explicit and dated.** An FX conversion produces a new `Money` plus the rate and date used; it never silently changes a stored amount.
5. **Allocation is exact.** Splitting an amount across parts uses largest-remainder allocation so the parts sum to the total, always.

## Consequences

**Positive** — balances are exact; splits and imports cannot drift; the serialised form is unambiguous; tests compare integers.

**Negative** — percentages, interest and multi-currency totals need explicit rounding decisions, which must be documented rather than emergent. Division by a non-factor requires a policy choice per call site.

**Neutral** — a small currency table (ISO 4217 exponent) must be maintained, including historical currencies.

## Follow-ups

- [ ] Property tests: random postings → balance equals independent integer sum.
- [ ] Test JPY (0 dp), USD (2 dp), KWD (3 dp).
- [ ] Document the rounding policy for budget rollover and FX conversion.
