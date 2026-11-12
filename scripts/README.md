# Scripts

Development and CI helpers. Nothing here ships in the app.

## Available

| Script | Purpose |
| --- | --- |
| `check.sh` | Format check, clippy and tests in one command — the same gate CI runs |

## Planned

| Script | Purpose |
| --- | --- |
| `check.ps1` | The same gate for a native Windows shell |
| `file-size` | Fail when a Rust file exceeds 800 non-test lines (Rune's rule, adopted) |
| `deps-check` | Verify `finance-core` has no I/O or UI dependencies |
| `gen-fixtures` | Regenerate synthetic import fixtures deterministically |
| `render-screens` | Headless Rune renders for visual review |

## Rules

- Scripts are cross-platform: PowerShell for Windows, POSIX shell for the rest, or a Rust binary if logic gets real.
- No script may touch a user's real dataset. Anything destructive works in a temp directory.
- Exit non-zero on failure and print the failing command, so CI logs are readable.
- Scripts are reviewed like code: same standards, same PRs.
