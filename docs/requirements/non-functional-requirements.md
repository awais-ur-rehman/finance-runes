# Non-functional requirements

**Status:** proposed — targets are starting points for the requirement session
**Last updated:** 2026-11-12

Non-functional requirements are the ones users feel but never ask for. Each has a **measurable target** and a **way to verify it**; a target nobody measures is a wish.

---

## Correctness and integrity

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-001 | Balance arithmetic is exact for every supported currency | Zero discrepancies across randomised ledgers | Property tests: random postings → balance equals independent sum |
| NFR-002 | Imported totals match the source file | Exact match on every fixture | Golden tests per bank fixture |
| NFR-003 | A crash or power loss during a write never leaves an unreadable dataset | 0 corruption cases in fault-injection runs | Kill -9 during write loop, then reopen |
| NFR-004 | Migrations are atomic and idempotent | Re-running a migration changes nothing | Migration tests from every historical schema |
| NFR-005 | Deleting or editing cannot orphan a posting | 0 orphans | Foreign-key / invariant checks in test suite |

## Performance

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-010 | Cold start to interactive register, 20k transactions | < 1.5 s | Benchmarked release build, recorded in PR |
| NFR-011 | Scroll a 20k-row register | 60 fps minimum, no visible hitching | Headless frame-time trace |
| NFR-012 | Search across 20k transactions | < 100 ms for first results | Bench with p95 reported |
| NFR-013 | Switch between top-level screens | < 50 ms | Frame trace |
| NFR-014 | Import a 5,000-row CSV | < 2 s including duplicate detection | Bench |
| NFR-015 | Memory for a 20k-transaction dataset | < 250 MB resident | `procwatch`-style gauge in a perf build |
| NFR-016 | Idle application draws nothing | 0 frames rendered while idle | Frame counter in debug overlay |

## Privacy and security

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-020 | No network access in normal operation | 0 outbound connections | Integration test with a network-blocking harness |
| NFR-021 | No telemetry, analytics or crash upload by default | 0 | Code review + dependency audit |
| NFR-022 | Data stays within a user-chosen directory | No writes outside it, except OS temp during an atomic swap | Filesystem-write audit test |
| NFR-023 | Imported files are parsed defensively | No panic on malformed input; fuzz targets run clean | `cargo-fuzz` targets per parser |
| NFR-024 | Secrets never appear in logs or error messages | 0 occurrences | Log-scrubbing test on representative errors |

## Portability and durability

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-030 | Runs on Windows, macOS and Linux | CI build + smoke test per platform | CI matrix |
| NFR-031 | Dataset is readable without the app | Documented export + schema doc | Manual check each release |
| NFR-032 | Forward compatibility is explicit | Opening a newer dataset refuses with a clear message, never silently downgrades | Version-gate test |
| NFR-033 | Backups restore to an identical dataset | Byte-level logical equality after restore | Round-trip test |

## Accessibility and usability

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-040 | Every action reachable by keyboard | 100% of primary flows | Keyboard-only test script |
| NFR-041 | Focus is visible and order follows reading order | No focus traps, no invisible focus | Manual + element-tree audit |
| NFR-042 | Text contrast meets WCAG AA | ≥ 4.5:1 body text, ≥ 3:1 large text | Automated contrast check on theme tokens |
| NFR-043 | UI scales with OS text size / DPI | No clipped labels at 150% | Render at 100/150/200% |
| NFR-044 | Colour is never the only carrier of meaning | 0 colour-only signals | Review of charts and states |

## Internationalisation

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-050 | Dates, numbers and currency formatting follow the locale | Correct for the test locale set | Formatting tests per locale |
| NFR-051 | Currencies with 0 or 3 decimal places work | JPY, KWD correct | Currency table tests |
| NFR-052 | UI strings are externalised | 0 hard-coded user-visible strings in view code | Lint / grep check |
| NFR-053 | Right-to-left layouts are not broken | Mirroring works or is explicitly unsupported and documented | Render test |

## Maintainability

| ID | Requirement | Target | Verified by |
| --- | --- | --- | --- |
| NFR-060 | Domain crate has no I/O, clock, randomness or UI dependency | 0 such dependencies | `cargo tree` check in CI |
| NFR-061 | Builds are warning-free | 0 warnings with `-D warnings` | CI |
| NFR-062 | Public API is documented | Every `pub` item has a doc comment | `#![warn(missing_docs)]` |
| NFR-063 | Files stay reviewable | ≤ 800 non-test lines per Rust file | Script check in CI |
| NFR-064 | Tests are fast enough to run on every save | Full unit suite < 10 s | CI timing |
