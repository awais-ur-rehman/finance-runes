# Security Policy

This is a personal finance application. A security bug here can expose someone's complete financial history, so we treat reports seriously and quietly.

## Reporting a vulnerability

**Do not open a public issue.**

Use GitHub's private vulnerability reporting on this repository
(**Security → Report a vulnerability**). If that is unavailable, contact the maintainer directly and say only that you have a security report — no details in a public channel.

Please include:

- affected version or commit,
- operating system,
- a description of the impact (what data or capability is exposed),
- reproduction steps or a proof of concept,
- any suggested fix.

## What to expect

| Stage | Target |
| --- | --- |
| Acknowledgement | 3 business days |
| Triage and severity assessment | 10 business days |
| Fix or documented mitigation | depends on severity; critical issues first |

We will credit you in the release notes unless you ask us not to.

## Scope

In scope:

- Unauthorised read, write or exfiltration of local data (database, exports, attachments, backups).
- Path traversal, unsafe deserialisation, injection in importers (CSV/OFX/QIF).
- Memory-safety issues, including in `unsafe` blocks and FFI.
- Weaknesses in at-rest encryption, key handling or backup integrity.
- Supply-chain issues in our dependency tree.

Out of scope:

- Attacks that require an already-compromised OS account or physical access to an unlocked machine.
- Social engineering.
- Denial of service against your own local database by editing it by hand.
- Reports produced solely by an automated scanner without a demonstrated impact.

## Our commitments

- **Local-first, no telemetry.** The app does not phone home. Any future network feature will be opt-in, documented, and off by default.
- **No secrets in the repository.** CI will scan for credentials; see [data privacy](docs/engineering/data-privacy-and-security.md).
- **Dependencies are pinned and reviewed.** `Cargo.lock` is committed; new dependencies need a reason in the PR.
- **Fail closed.** A corrupt or unreadable database is reported, never silently rewritten.

## Supported versions

Pre-1.0: only the latest release and `main` receive fixes.
