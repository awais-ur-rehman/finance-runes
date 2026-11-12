# Pull request

## What and why

<!-- One paragraph. What changes, and what problem it solves. Link the issue: Closes #123 -->

## Requirement

<!-- Requirement IDs this implements, e.g. FR-012, NFR-004. Write "docs only" or "no requirement (chore)" if none. -->

## How

<!-- The approach, and anything a reviewer would otherwise have to reverse-engineer. Call out trade-offs you rejected. -->

## Data impact

<!-- Does this change stored data, a migration, an importer, or a backup format? If yes, describe forward/backward compatibility and rollback. If no, say "none". -->

## Checklist

- [ ] Tests cover the new behaviour, including failure paths
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] No real financial data, secrets, credentials or machine paths in the diff
- [ ] No new dependency, or the reason is stated above
- [ ] Docs / ADR / `CHANGELOG.md` updated where behaviour changed
- [ ] No unrelated reformatting

## Screenshots

<!-- UI changes: include a headless render or a screenshot. Before/after if the change is visual. -->
