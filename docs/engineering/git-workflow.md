# Git workflow

**Status:** active
**Last updated:** 2026-11-12

## Branching

Trunk-based. `main` is always green and always releasable. Work happens on short-lived branches that live for hours or days, not weeks.

```text
main ──●──●──●──●──●──●──►
        \    /   \   /
         ●──●     ●─●      feat/…, fix/…
```

- No `develop` branch, no release branches for now.
- No long-running feature branches. If a feature cannot land in a week, land it dark behind a compile-time flag or split it into smaller changes.
- `main` is protected: PR required, CI green, one approval.

### Branch names

`<type>/<short-slug>`, lowercase, hyphens:

```text
feat/transaction-splitting
fix/import-duplicate-detection
docs/requirements-budgets
refactor/money-newtype
test/ledger-property-tests
perf/register-virtualisation
chore/pin-rune-rev
```

## Commits

[Conventional Commits](https://www.conventionalcommits.org/). The subject is imperative, lowercase, no trailing period, ≤ 72 characters.

```text
feat(core): reject transfers between the same account
fix(import): treat parenthesised amounts as negative
docs(adr): accept ADR-0005 ledger model
refactor(storage): move row mapping into its own module
test(money): add property test for split allocation
chore(deps): pin rune to dev rev 3f2a91c
```

Allowed types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`, `revert`.

Scopes follow the crate or area: `core`, `storage`, `ui`, `import`, `budget`, `report`, `adr`, `deps`.

### What a commit contains

- One logical change.
- The tests for that change.
- The doc update for that change, if behaviour changed.

Do not mix: a rename + a bug fix, or a dependency bump + a feature. Reviewers cannot review two things at once, and `git bisect` cannot bisect a mixture.

### Writing the body

Explain **why**, not what — the diff already shows what.

```text
fix(import): treat parenthesised amounts as negative

Some banks write debits as (120.00). The parser accepted the number and
dropped the sign, so imported expenses became income. Parenthesised values
are now negated before parsing, with a regression fixture from the same
format family.
```

## Pull requests

- Title = the squash commit subject.
- Body fills the template: what/why, requirement ID, approach, data impact, checklist.
- Keep it reviewable: aim for under ~400 changed lines. Larger PRs need a reason in the description.
- Self-review the diff before requesting review. Read every line.
- A PR that changes stored data, a migration or an importer must say what happens to existing datasets and how to roll back.
- UI PRs include a headless render or screenshot.

## Merging

- Squash merge. The PR title becomes the commit.
- Delete the branch after merge.
- Never force-push `main`. Never rewrite public history.
- If `main` moved, rebase your branch; do not merge `main` into it (keeps history linear).

## Releases

- SemVer. Pre-1.0, `0.x.y`: breaking changes bump the minor.
- Tag `vX.Y.Z` on `main` with an annotated tag.
- Update `CHANGELOG.md` before tagging: move `Unreleased` entries under the new version with a date.
- A release notes entry says what the user can now do, not which module changed.

## Changelog

`CHANGELOG.md` follows Keep a Changelog. Add to `Unreleased` in the PR that changes behaviour:

- `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
- One line per user-visible change. Internal refactors are omitted unless they change behaviour or performance noticeably.

## Repository hygiene

- `Cargo.lock` is committed (this is an application).
- Generated files, build output and datasets are ignored — see `.gitignore`.
- Large binaries are not committed; assets are small and purposeful.
- **Never commit** personal data, secrets, or machine-specific paths. Before pushing:

```bash
git diff --cached | rg -n "secret|token|password|/Users/|C:\\\\Users|BEGIN .*PRIVATE KEY"
```

## Undo

- Unpushed commit: `git reset --soft HEAD~1`.
- Pushed on a branch: `git revert` and push the revert; do not force-push a shared branch.
- Accidental secret: rotate it first, then tell the maintainer. Removing it from history does not un-leak it.
