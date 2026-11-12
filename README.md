# Finance

A local-first personal finance manager — accounts, transactions, budgets and reports — built as a native desktop app in Rust on [Rune](https://github.com/anomalyco/rune).

> **Status: pre-requirements.** This repository currently holds the project scaffold, documentation and engineering conventions. No application code has been written. Requirements are gathered next; see [Roadmap](#roadmap).

---

## Why

Personal finance tools force a bad trade: either your financial history lives in someone else's cloud, or you maintain a spreadsheet that rots. This project takes the third option — a fast native app whose data stays on your machine, in a format you can read, with the rigour of a real ledger underneath the UI.

Two beliefs shape every decision here:

1. **Money is data, not decoration.** Balances are derived from an append-only ledger of transactions. Nothing is silently rounded, and nothing is overwritten.
2. **Your data outlives the app.** Storage is local, exportable, and readable without this binary.

## What it will do (draft)

This is intent, not a specification. It will be replaced by [the requirements documents](docs/requirements/) after requirement gathering.

- **Accounts** — checking, savings, credit cards, cash, loans, tracked in any currency.
- **Transactions** — manual entry, splitting, transfers between accounts, notes and attachments.
- **Import** — CSV/OFX/QIF bank exports with duplicate detection and a review step.
- **Categories & budgets** — hierarchical categories, monthly budgets, rollover rules.
- **Reports** — net worth over time, cash flow, spending by category, budget variance.
- **Local-first** — offline by default, single-user, no account required.

### Explicit non-goals for v1

- Multi-user or team access.
- Bank API aggregation / open banking integrations.
- Investment portfolio tracking and market data.
- Tax filing or advice.
- Mobile clients (the Rune UI is desktop-first; the domain core stays portable).

## Stack

| Layer | Choice | Notes |
| --- | --- | --- |
| Language | Rust 1.99.1 | Pinned in [`rust-toolchain.toml`](rust-toolchain.toml) |
| UI | [Rune](https://github.com/anomalyco/rune) + `rune-kit` | Pre-release; pinned to a `dev` rev. Never `dev` HEAD floating. |
| Storage | TBD — see [ADR-0004](docs/architecture/decisions/0004-local-first-storage.md) | Local-first, portable, migratable |
| Platform | macOS, Windows, Linux | Rune renders natively via wgpu/winit |

Rune is pre-release. Apps pin a rev of `dev`; `dev` is never rewritten, so a pinned rev keeps building. The rev is recorded in the workspace manifest when the first crate lands.

## Repository layout

```text
.
├── docs/                     # Everything that defines the product before code
│   ├── 00-project-brief.md   # Problem, users, scope, success criteria
│   ├── requirements/         # Functional, non-functional, glossary, open questions
│   ├── architecture/         # Layering, boundaries, ADRs
│   ├── engineering/          # Coding standards, testing, git workflow, security
│   └── design/               # UI principles and Rune-specific guidance
├── crates/                   # Rust workspace members (empty until requirements close)
│   ├── finance-core/         # Domain: money, ledger, budgets. No I/O, no UI.
│   ├── finance-storage/      # Persistence adapters behind core traits
│   └── finance-ui/           # Rune views, theming, input handling
├── assets/                   # Icons, fonts, sample (anonymised) fixtures
├── scripts/                  # Dev and CI helper scripts
└── .github/                  # Issue and PR templates
```

Each directory has its own `README.md` stating its responsibility and dependency rules.

## Getting started

Nothing to build yet. To prepare:

```bash
git clone <this repo>
cd Finance
rustup toolchain install 1.99.1
rustc --version   # expect 1.99.1
```

Once the workspace exists, the expected entry points will be:

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Documentation

Start at [`docs/README.md`](docs/README.md) for the full index. The short version:

- **What we're building and why** → [project brief](docs/00-project-brief.md)
- **How we'll work** → [coding standards](docs/engineering/coding-standards.md), [git workflow](docs/engineering/git-workflow.md)
- **Why the architecture looks like this** → [ADRs](docs/architecture/decisions/)
- **What's still undecided** → [open questions](docs/requirements/open-questions.md)

## Engineering principles

1. **The domain has no dependencies.** `finance-core` knows nothing about SQL, files, Rune or the clock. Time and IDs are injected.
2. **Money is never a float.** Amounts are integers in minor units plus a currency code. See [ADR-0003](docs/architecture/decisions/0003-money-and-currency.md).
3. **Balances are derived.** The ledger is append-only; balances are a projection, so a bug in a projection cannot destroy history.
4. **Every change is reversible.** Migrations and imports are transactional and previewed before they commit.
5. **Tests are the specification.** A requirement that is not covered by a test is not done.
6. **Small files, small PRs.** Rust files stay under 800 non-test lines; a folder is reshaped before it grows past ~40 source files.
7. **No secrets, no personal data, ever.** Not in commits, not in fixtures, not in logs.

## Roadmap

| Phase | Deliverable | State |
| --- | --- | --- |
| 0 | Repository scaffold, conventions, docs skeleton | ✅ done |
| 1 | Requirement gathering → filled `docs/requirements/` | ⬜ next |
| 2 | Architecture sign-off (ADRs accepted), workspace created | ⬜ |
| 3 | `finance-core` domain model + property tests | ⬜ |
| 4 | Storage adapter + migrations + importers | ⬜ |
| 5 | Rune UI: accounts, register, budgets, reports | ⬜ |
| 6 | Packaging for macOS / Windows / Linux | ⬜ |

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a branch. Work is trunk-based on `main`, commits follow Conventional Commits, and every PR is small enough to review in one sitting.

## License

MIT — see [`LICENSE`](LICENSE).

## Security

Please read [`SECURITY.md`](SECURITY.md) before reporting a vulnerability. Do not open a public issue for anything that could expose a user's financial data.
