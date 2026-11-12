# Contributing

Thanks for caring about this project. This document is short on ceremony and long on the things that actually keep a finance codebase trustworthy.

## Before you write code

**We are in requirement gathering.** No application code is merged until the requirements in [`docs/requirements/`](docs/requirements/) describe the behaviour and the relevant ADRs are accepted. If you have an idea, open an issue using the feature request template, or add it to [`docs/requirements/open-questions.md`](docs/requirements/open-questions.md).

## Ground rules

1. **Never commit real financial data.** Not in fixtures, not in tests, not in screenshots, not in logs. Fixtures are synthetic or anonymised beyond recognition. See [`docs/engineering/data-privacy-and-security.md`](docs/engineering/data-privacy-and-security.md).
2. **Never commit secrets.** No tokens, keys, `.env` files or credential dumps. If you think you leaked one, see [`SECURITY.md`](SECURITY.md) — rotate first, then tell us.
3. **The domain is sacred.** `finance-core` must not depend on storage, UI, filesystem, network or wall-clock time. If your change needs any of those, it belongs in an adapter.
4. **Money is never `f64`.** Use the `Money` type from `finance-core`.
5. **One logical change per PR.** If the diff needs a table of contents, split it.

## Workflow

```bash
git switch main
git pull --ff-only
git switch -c feat/transaction-splitting

# ... work ...

cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Branches: `feat/…`, `fix/…`, `docs/…`, `chore/…`, `refactor/…`, `test/…`, `perf/…`.
- Commits: [Conventional Commits](https://www.conventionalcommits.org/) — `feat(core): reject transfers between the same account`.
- Merge: squash to `main`, with the PR title as the commit subject.
- Detail: [`docs/engineering/git-workflow.md`](docs/engineering/git-workflow.md).

## Definition of done

A change is done when all of these are true:

- [ ] Behaviour is described by a requirement ID (`FR-###` / `NFR-###`) or the PR explains why none applies.
- [ ] Tests cover the new behaviour, including the failure paths.
- [ ] `cargo fmt`, `clippy -D warnings` and the full test suite pass locally.
- [ ] Public items have doc comments explaining *why*, not restating the signature.
- [ ] No new file exceeds the size caps in [coding standards](docs/engineering/coding-standards.md).
- [ ] User-visible change is noted in [`CHANGELOG.md`](CHANGELOG.md) under `Unreleased`.
- [ ] No secrets or personal data in the diff (`git diff --cached` read end to end).

## Review etiquette

- Reviewers comment on the code, never the person.
- "Nit:" means take it or leave it. Everything else is a request.
- Approve when the change is *correct and maintainable*, not when it is perfect.
- Authors resolve their own threads; reviewers re-read resolved threads before merging.

## Reporting bugs

Use the bug report template. A useful report includes: what you did, what you expected, what happened, the app version, the OS, and whether the bug touches stored data. **Do not attach your database or real statements** — reproduce with a synthetic fixture instead.

## License

By contributing you agree that your contribution is licensed under the [MIT License](LICENSE).
