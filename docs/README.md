# Documentation

Everything that defines the product before, and alongside, the code. If a decision is not written down here, it is not a decision.

## Read in this order

| # | Document | What it answers |
| --- | --- | --- |
| 1 | [Project brief](00-project-brief.md) | What are we building, for whom, and why? |
| 2 | [Requirements index](requirements/README.md) | How requirements are written and tracked |
| 3 | [Functional requirements](requirements/functional-requirements.md) | What the app must do |
| 4 | [Non-functional requirements](requirements/non-functional-requirements.md) | How well it must do it |
| 5 | [Glossary](requirements/glossary.md) | What our words mean |
| 6 | [Open questions](requirements/open-questions.md) | What is still undecided |
| 7 | [Architecture](architecture/README.md) | How the system is shaped, and why |
| 8 | [Coding standards](engineering/coding-standards.md) | How we write Rust here |
| 9 | [Testing strategy](engineering/testing-strategy.md) | How we prove it works |
| 10 | [Git workflow](engineering/git-workflow.md) | How changes land |
| 11 | [Data, privacy and security](engineering/data-privacy-and-security.md) | How we protect the one thing that matters |
| 12 | [Design](design/README.md) | How it looks and feels |

## Conventions

- **Docs are versioned with the code.** A behaviour change updates the doc in the same PR.
- **Dates and status.** Every living document starts with a status line and a last-updated date.
- **Decisions are immutable once accepted.** To change one, write a new ADR that supersedes the old one; never edit history.
- **No real financial data.** Examples use invented numbers and `Example Bank`.

## Folder map

```text
docs/
├── 00-project-brief.md
├── requirements/     # FR/NFR, glossary, open questions
├── architecture/     # layering, boundaries, ADRs
│   └── decisions/    # ADR-NNNN
├── engineering/      # standards, testing, workflow, security
└── design/           # UI principles, Rune guidance
```
