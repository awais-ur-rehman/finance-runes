# Requirements

**Status:** template — to be filled during requirement gathering
**Last updated:** 2026-11-12

## How we write requirements here

A requirement is a statement about **observable behaviour**, written so that a test could fail it. If you cannot imagine the failing test, it is not a requirement yet — it is an idea. Ideas go in [open questions](open-questions.md).

### Identifiers

| Prefix | Meaning | Example |
| --- | --- | --- |
| `FR-###` | Functional requirement | `FR-014` |
| `NFR-###` | Non-functional requirement | `NFR-003` |
| `Q-##` | Open question | `Q-07` |
| `ADR-####` | Architecture decision | `ADR-0003` |

IDs are **never reused or renumbered**. A dropped requirement keeps its ID and is marked `Dropped` with a reason, because old commits and PRs still reference it.

### Format

```markdown
### FR-014 — Split a transaction across categories

**Priority:** Must
**Status:** Proposed | Accepted | Implemented | Dropped
**Depends on:** FR-003
**Source:** requirement session 2026-11-20

A transaction can be divided into two or more parts, each with its own category
and amount. The parts must sum exactly to the transaction total; the app refuses
to save a split that does not.

**Acceptance criteria**

- Given a −120.00 transaction, when I split it into −80.00 Groceries and −40.00 Household, then both parts save and the account balance is unchanged.
- Given a split whose parts sum to −119.99, when I save, then the app shows the 0.01 difference and refuses to save.
```

### Rules

1. **One behaviour per requirement.** "Import CSV and categorise it" is two requirements.
2. **Priorities are MoSCoW**: Must / Should / Could / Won't (this release).
3. **Acceptance criteria are Given/When/Then** and cover the failure path, not only the happy path.
4. **Money in examples is always minor-unit exact** and uses invented data.
5. **Every accepted requirement must be traceable** to a test name or a PR. Untraceable requirements are either untested or imaginary.

## Documents

| File | Contents |
| --- | --- |
| [functional-requirements.md](functional-requirements.md) | What the app does, by area |
| [non-functional-requirements.md](non-functional-requirements.md) | Quality attributes with measurable targets |
| [glossary.md](glossary.md) | Domain vocabulary — read this before writing requirements |
| [open-questions.md](open-questions.md) | Undecided items blocking or shaping requirements |

## Process

1. **Elicit.** Walk the user's real workflow: a typical month, an unusual month, a mistake, a year-end.
2. **Write.** Draft requirements as `Proposed`.
3. **Challenge.** For each: what breaks it? What is the failure path? What does it cost?
4. **Accept.** Move to `Accepted` only when the acceptance criteria are testable and the open questions it depends on are resolved.
5. **Trace.** Link accepted requirements from the PR that implements them.

## Definition of a good requirement

- A new team member could implement it without asking a question.
- A tester could verify it without reading the source.
- It says what happens when the input is wrong.
- It does not name a library, a table or a widget.
