# ADR-0001 — Record architecture decisions

**Status:** accepted
**Date:** 2026-11-12
**Deciders:** project owner
**Supersedes:** —

## Context

This project will accumulate decisions that are expensive to reverse: the ledger model, the storage engine, how money is represented, how imports are committed. Each is easy to explain at the moment it is made and impossible to reconstruct six months later from the code alone.

Code shows *what* we did. It does not show what we rejected, or why the obvious alternative was wrong for us.

## Decision

We record every architecturally significant decision as a numbered ADR in `docs/architecture/decisions/`, using the template in `ADR-0000`.

An ADR is required when a change:

- introduces or removes a dependency that the domain relies on,
- changes a persisted data format or a migration strategy,
- changes a crate boundary or the direction of a dependency,
- chooses between two reasonable technical approaches with lasting cost,
- sets a rule that reviewers must enforce (e.g. "no floats for money").

An ADR is **not** required for naming, local refactors, or choosing a helper function.

## Consequences

**Positive** — reviewers have a place to point; new contributors learn the reasoning; we stop relitigating settled questions.

**Negative** — writing takes time, and stale ADRs can mislead. Mitigation: every ADR carries a status, and a superseded ADR links forward to its replacement.

**Neutral** — small ongoing documentation cost.

## Follow-ups

- [x] Template created.
- [ ] Add an ADR check to the PR template (done: the template asks for requirement/decision links).
