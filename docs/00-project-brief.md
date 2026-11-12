# Project brief

**Status:** requirement gathering complete — 5 of 5 sessions
**Last updated:** 2026-11-12
**Owner:** awais-ur-rehman

## 1. Problem

Tracking personal spending in Pakistan usually means one of three bad options: a spreadsheet that has no structure and answers no questions, a cloud budgeting app built around US/EU bank feeds that does not match how people actually pay here, or nothing at all — so the month ends with no idea where the money went.

The user's own words for what they want:

> "I want to add what and where I am spending, what are my expenses monthly, then I will input and the system will keep the record and also tell me what to do in order to save money."

So the product has three jobs, in this order:

1. **Capture** an expense quickly, with enough context to be useful later.
2. **Summarise** the month clearly — total, trend, categories, savings rate.
3. **Advise** concretely: what to change to spend less, based on the user's own numbers.

## 2. Target user

A single individual in Pakistan who:

- keeps money in **PKR** across a bank account and cash,
- **enters transactions manually**, with a **one-time CSV import** to seed the last three months,
- wants monthly clarity and actionable advice, not accounting,
- uses a desktop machine and expects the app on Windows, macOS and Linux,
- values owning their data locally.

They are not an accountant. They do not want to learn debits, credits, ledgers or reconciliation vocabulary. They want to know: *where did my money go, and what should I change?*

## 3. Value proposition

> Record an expense in seconds, see the month at a glance, and get specific, explainable suggestions for spending less.

Differentiators:

1. **Built for manual entry.** Most trackers assume a bank feed. This one is designed around typing an expense in under ten seconds, keyboard-first.
2. **Advice, not just charts.** Every insight names the category, the amount, the comparison period and the action — no generic "spend less on coffee".
3. **Yours.** Local database, documented export, no account, no telemetry.

## 4. Scope

### In scope for v1

- **Manual expense and income entry** — amount, date (default today), category, merchant/source, account, optional note.
- **Accounts** — bank and cash with UI; savings, card and wallet kinds exist in the model.
- **Transfers** between own accounts, excluded from income and expense totals.
- **Categories** — two levels, seeded with an editable starter set.
- **Search** by merchant, category, account, date range and amount range.
- **CSV import** — one-time historical load: column mapping, review with category assignment, duplicate detection, atomic commit.
- **Budgets** — monthly limit per category, with overspend and near-limit flags.
- **Month-end surplus** — unspent budget offered as a *confirmed* transfer to a savings account.
- **Recurring expenses** — templates with a due date and one-action posting.
- **Savings goal** — monthly target with progress.
- **Month view** — total spend and trend, savings rate, category breakdown.
- **Savings insights** — deterministic, explainable findings plus a month-end summary.
- **Backup and export** — automatic on exit plus manual, and a documented export.

### Out of scope for v1

- **Ongoing monthly import** — v1 imports once to seed history; re-import is not a v1 workflow.
- **OFX/QFX/QIF import** — CSV only.
- **Multi-currency** — PKR only. Amounts still carry a currency code so this can be added later.
- Splitting one expense across categories.
- Multi-user, sharing, cloud sync.
- Bank API / open banking aggregation.
- Investments, securities, market data.
- Tax filing or advice.
- Mobile and web apps.
- Right-to-left layout.
- Encryption at rest beyond OS disk encryption.

### Deferred, but designed for

- Monthly re-import, OFX/QIF and wallet statement formats.
- Multi-currency and FX rates.
- Splitting transactions.
- Receipt attachments.
- Automatic posting of recurring items.
- Conversational assistant over the insights engine (ADR-0006).
- Multiple datasets.
- Multi-device sync (only if local-first can be preserved).

## 5. Decisions so far

| # | Question | Decision | Consequence |
| --- | --- | --- | --- |
| 1 | Primary goal | Capture spending, monthly summary, **savings advice** | Advice becomes a v1 feature, not a nice-to-have |
| 2 | Currency | **PKR only** | No FX or conversion UI; money still carries a currency code |
| 3 | Input method | **Manual entry**, plus a **one-time CSV import** | Import stays narrow: one file, one review, no ongoing sync |
| 4 | Platform | **Windows, macOS, Linux equally** | CI matrix and packaging from day one |
| 5 | Ledger model (Q-01) | **Hidden double-entry** (ADR-0005) | Transfers are provably correct; UI shows no accounting terms |
| 6 | Encryption (Q-04) | **OS disk encryption only** | Documented plainly; no key-recovery burden |
| 7 | Accounts (Q-20) | **Bank account and cash** | Wallet and card kinds exist but need no UI |
| 8 | Income (Q-17) | **Expenses and income** | Savings rate and "what's left" become real numbers |
| 9 | Entry fields (Q-22) | **Amount, category, merchant, account** (+ date) | Four required fields; note optional |
| 10 | "Where" (Q-21) | **Merchant/payee + free-text note** | Merchant-level reporting; no tags or location |
| 11 | Budgets (Q-18) | **Monthly limit per category** | Advice can be concrete: over by an exact amount |
| 12 | Advice (Q-19) | **Rule-based insights now, assistant later** | v1 stays offline and deterministic; a seam is kept (ADR-0006) |
| 13 | Categories | **Two levels**, seeded starter set | Fast entry, readable reports |
| 14 | Splitting | **Not in v1** | One expense, one category; the model can grow later |
| 15 | Correction | **Edit + delete with immediate undo** | No undo engine to maintain |
| 16 | Search | Merchant, category, account, date range, amount | Note-text search deferred |
| 17 | Duplicates | **Warn, never block** | Manual entry makes double-taps likely |
| 18 | Budget meaning (Q-23) | **Cap**; unspent offered as a confirmed transfer | No phantom money; the ledger stays true |
| 19 | Month view | Total + trend, savings rate, category breakdown | Merchant breakdown is a secondary screen |
| 20 | Advice delivery | **Insights screen + month-end summary** | No constant nagging |
| 21 | History (Q-24) | One-time CSV import, ~3 months | Advice has a baseline from day one |
| 22 | Backup (Q-25) | Auto on exit + manual, keep 10 | |
| 23 | Formatting (Q-26) | **South Asian grouping** default | `12,34,567`, configurable |
| 24 | Packaging (Q-27) | **Portable archive** per platform | No signing or installer in v1 |
| 25 | Language (Q-28) | English, strings externalised | Urdu later is translation, not refactor |

## 6. Success criteria

| Criterion | Target |
| --- | --- |
| Record one expense | under 10 s first time, under 5 s thereafter; keyboard only |
| Cold start to month view | under 1.5 s on a 5-year dataset |
| Month view | renders under 100 ms; scrolls at 60 fps |
| Import three months | under 2 s for 500 rows, with duplicate detection |
| Advice quality | every finding states amount, subject, comparison and action |
| Advice with thin data | useful from month one; no pattern claims without three months |
| Month-end surplus | offered total always traceable to per-category lines |
| Data durability | a crash mid-write never leaves an unreadable dataset |
| Portability | export readable without this app; backup restores identically |
| Privacy | zero network calls in normal operation, verified by test |

## 7. Constraints and assumptions

- **Rune is pre-release.** Pin a `dev` rev; keep the UI layer thin.
- **Solo developer, agent-assisted.** Boring solutions, explicit decisions, small PRs.
- **Three platforms is a real cost.** CI runs all three from the first crate; no OS-specific code in the domain.
- **Manual entry makes UX the product.** If entry is slow, nothing else matters.
- **The CSV shape is unknown.** The importer must work from a generic mapping UI until a real sample arrives.
- **Advice needs data.** A new user has three imported months at best; the first month must still be useful.

## 8. Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Entry friction → user stops recording | Fatal: no data, no value | Keyboard-first entry, smart defaults, four fields, fast correction |
| Advice feels generic or preachy | Trust loss | Data-backed findings only; quantified; dismissible; no moralising copy |
| Advice from thin data is wrong | Credibility loss | Minimum-history rules; state what is missing (FR-106) |
| Unknown CSV shape breaks the one-time import | Bad history, wasted effort | Generic mapping, preview, atomic commit, saved profile once a sample exists (Q-29) |
| Surplus-to-savings creates phantom money | Ledger lies | Transfer only after explicit confirmation; no automatic posting (FR-053) |
| Three-platform support slows delivery | Schedule | CI matrix early; portable archives; no platform code in the domain |
| PKR-only assumption breaks | Rework | Currency code in the money type from day one |
| Scope creep back into full importers | Delay | Ongoing import is explicitly out of v1 |

## 9. Open decisions

- **Q-02** — storage engine. SQLite recommended, ADR-0004 still needs a yes/no.
- **Q-29** — the real CSV columns. Blocks the importer's saved profile, not the mapping UI.
- **Q-30** — how a savings account comes into existence.
- **Q-31** — whether month close is explicit or automatic.
- **Q-32** — the exact unspent-budget formula, including categories without a limit.
- **Q-34** — whether deletes are soft (recommended) or hard.

## 10. Session log

| Session | Date | Focus | Outcome |
| --- | --- | --- | --- |
| 1 | 2026-11-12 | Context and scope | Goal, currency, input, platform, ledger, encryption |
| 2 | 2026-11-12 | Accounts, entry, "where" | Bank + cash, income, four fields, merchant + note, budgets, advice model |
| 3 | 2026-11-12 | Categories, correction, lookup | Two levels, starter set, no splits, edit/delete with undo, search scope, duplicate warning |
| 4 | 2026-11-12 | Budgets, month view, advice | Cap + confirmed surplus transfer, month view contents, recurring, savings goal, insight types |
| 5 | 2026-11-12 | History, safety, delivery | One-time CSV import, backup policy, South Asian formatting, portable zip, English-only i18n-ready |
