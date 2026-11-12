# Non-functional requirements

**Status:** accepted for v1 — 2026-11-12
**Last updated:** 2026-11-12

Each target is measurable and has a way to verify it. A target nobody measures is a wish.

Scope note: v1 is PKR-only, single-user, desktop, manual entry plus a one-time CSV import.

---

## Correctness and integrity

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-001 | Balance arithmetic is exact | Zero discrepancies across randomised ledgers | Property test: random postings → balance equals an independent integer sum |
| NFR-002 | Imported totals match the source file | Exact match on every fixture | Golden tests per bank profile |
| NFR-003 | A crash during a write never leaves an unreadable dataset | 0 corruption cases in fault injection | Kill the process mid-write, reopen, run integrity check |
| NFR-004 | Migrations are atomic and idempotent | Re-running a migration changes nothing | Migration tests from every historical schema |
| NFR-005 | Editing or deleting cannot orphan a posting | 0 orphans | Invariant checks in the test suite |
| NFR-006 | Month totals equal the sum of their transactions | Exact, for every month | Property test over generated ledgers |

## Performance

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-010 | Keystroke-to-screen feedback in the entry form | < 16 ms | Frame trace in a debug overlay |
| NFR-011 | Saving a transaction | < 50 ms, form ready for the next entry | Bench, release build |
| NFR-012 | Month view render | < 100 ms on 20k transactions | Bench |
| NFR-013 | Register scroll | 60 fps, no visible hitching | Headless frame-time trace |
| NFR-014 | Search across 20k transactions | < 100 ms to first results | Bench, p95 reported |
| NFR-015 | Import 500 rows with duplicate detection | < 2 s | Bench |
| NFR-016 | Cold start to interactive month view | < 1.5 s on a 5-year dataset | Bench, release build |
| NFR-017 | Idle application draws nothing | 0 frames while idle | Frame counter |
| NFR-018 | Memory for a 20k-transaction dataset | < 250 MB resident | Perf-build gauge |

## Privacy and security

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-020 | No network access in normal operation | 0 outbound connections | Integration test with a network-blocking harness |
| NFR-021 | No telemetry, analytics or crash upload | 0 | Code review and dependency audit |
| NFR-022 | Writes stay inside the chosen data directory | No writes elsewhere except an OS temp file during an atomic swap | Filesystem-write audit |
| NFR-023 | The CSV parser never panics on malformed input | 0 panics; row-level errors instead | Fuzz target |
| NFR-024 | No amounts, merchants, notes or paths in logs | 0 occurrences | Log-scrubbing test |
| NFR-025 | Imported files cannot escape the data directory | No path traversal | Adversarial filename tests |

## Portability and durability

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-030 | Runs on Windows, macOS and Linux | Build and smoke test on all three | CI matrix on every PR |
| NFR-031 | Ships as a portable archive per platform | Extract and run, no installer | Release checklist |
| NFR-032 | Dataset is readable without the app | Documented CSV export plus schema doc | Manual check each release |
| NFR-033 | Backups restore to an identical dataset | Logical equality after restore | Round-trip test |
| NFR-034 | A newer dataset is refused, never downgraded | Clear message naming both versions | Version-gate test |

## Accessibility and usability

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-040 | Every primary flow works with the keyboard alone | 100%: record, edit, delete, import, set budget | Keyboard-only script |
| NFR-041 | Focus is always visible and follows reading order | No traps, no invisible focus | Element-tree audit |
| NFR-042 | Contrast meets WCAG AA | ≥ 4.5:1 body, ≥ 3:1 large text | Automated check on theme tokens |
| NFR-043 | UI scales with DPI and OS text size | No clipped labels at 150% and 200% | Renders at 100/150/200% |
| NFR-044 | Colour is never the only signal | 0 colour-only states | Review of charts, budgets, errors |
| NFR-045 | Recording one expense | < 10 s for a first-time user, < 5 s thereafter | Timed usability script |

## Formatting and internationalisation

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-050 | Large amounts use South Asian grouping by default | `12,34,567.00` | Formatting tests |
| NFR-051 | Grouping style is a setting | International and South Asian both correct | Formatting tests |
| NFR-052 | PKR renders with 2 decimal places and the correct symbol | `Rs 1,234.00` or configured form | Formatting tests |
| NFR-053 | The currency table supports 0, 2 and 3 decimal places | JPY, PKR, KWD correct in the domain | Currency table tests |
| NFR-054 | Dates follow a configurable pattern, defaulting to a clear unambiguous form | No ambiguous `03/04/2026` without a label | Formatting tests |
| NFR-055 | UI strings are externalised | 0 hard-coded user-visible strings in views | Grep check in CI |
| NFR-056 | Right-to-left is explicitly out of scope for v1 | Documented | README |

## Maintainability

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-060 | The domain crate has no I/O, clock, randomness or UI dependency | 0 such dependencies | `cargo tree` check in CI |
| NFR-061 | Warning-free builds | 0 warnings with `-D warnings` | CI |
| NFR-062 | Public API is documented | Every `pub` item has a doc comment | `#![warn(missing_docs)]` |
| NFR-063 | Files stay reviewable | ≤ 800 non-test lines per Rust file | Script check in CI |
| NFR-064 | Unit tests run on every save | Full unit suite < 10 s | CI timing |
| NFR-065 | One command runs the same checks as CI | `scripts/check` passes locally | Script exists and is used |

## Explicitly out of scope

- Multi-currency, FX rates and revaluation (NFR-053 keeps the domain ready).
- Sync, multi-device, multi-user.
- Encryption at rest beyond OS disk encryption (Q-04).
- Mobile and web builds.
- Right-to-left layouts (NFR-056).
