# AGENTS.md

Instructions for coding agents working in this repository. Humans should read [`CONTRIBUTING.md`](CONTRIBUTING.md) instead; the rules are the same.

## Current phase: requirement gathering

**Do not write application code.** There is no `Cargo.toml` and no `src/` on purpose. Work in this phase is:

- refining [`docs/00-project-brief.md`](docs/00-project-brief.md),
- filling in [`docs/requirements/`](docs/requirements/),
- recording decisions in [`docs/architecture/decisions/`](docs/architecture/decisions/).

If asked to build a feature before requirements exist, say so, then propose the requirement text and acceptance criteria first.

## Read before acting

| Task | Read first |
| --- | --- |
| Any Rust work | [`docs/engineering/coding-standards.md`](docs/engineering/coding-standards.md) |
| Domain or storage work | [`docs/architecture/README.md`](docs/architecture/README.md) + ADRs |
| UI work | [`docs/design/README.md`](docs/design/README.md), then Rune's `crates/rune/README.md` |
| Anything touching data | [`docs/engineering/data-privacy-and-security.md`](docs/engineering/data-privacy-and-security.md) |

## Hard rules

1. **Never** add real financial data, real names, bank statements or account numbers to the repository. Fixtures are synthetic.
2. **Never** commit secrets, `.env` files, tokens or machine paths.
3. Money is integer minor units + currency code. No `f64` arithmetic on money.
4. `finance-core` must stay free of I/O, UI, clock and randomness — inject them.
5. Do not add a dependency without stating why in the PR body.
6. Do not reformat unrelated files in a change.

## Working with Rune

Rune is pre-release and agent-oriented: apps can run headless, render to PNG, dump the element tree and receive scripted input.

- Pin the Rune `dev` rev; never float on `dev` HEAD. Record the rev in the workspace manifest.
- Check UI work headlessly — render a PNG and inspect the element tree — instead of assuming a layout is correct.
- Respect Rune's performance rules: start slow work after the first frame, build only mounted rows, bound every cache, and never do file I/O on the UI thread during frames.
- Rune's own conventions worth copying: files under 800 non-test lines, tests in sibling `tests.rs` files, explicit state machines instead of clusters of booleans, `mod`/`pub use` lists one item per line and sorted.

## Verification before you claim done

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Report exactly which commands you ran and their outcome. If a check could not run, say why. Do not describe code you did not write or tests you did not run as if they passed.

## Commit style

Conventional Commits, one logical change per commit. Example: `docs(requirements): add budget rollover questions`.
