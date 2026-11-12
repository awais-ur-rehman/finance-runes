# Crates

The Rust workspace, split so that a change in one concern cannot silently break another. **No crate exists yet** — these directories hold the intended shape until requirement gathering closes.

| Crate | Purpose | Status |
| --- | --- | --- |
| [`finance-core`](finance-core/README.md) | Domain model and pure calculations. The source of truth for rules. | Planned |
| [`finance-storage`](finance-storage/README.md) | Persistence, migrations, backup, integrity. | In progress |
| [`finance-ui`](finance-ui/README.md) | Rune views, theming, input, formatting. | Planned |
| `finance-app` | Binary / composition root: wires adapters, opens the window. | Planned |

## Dependency direction

```text
finance-app ──► finance-ui ──► finance-core
     │              │
     └──────────────┴──► finance-storage ──► finance-core
```

Arrows mean "may depend on". The rules:

- `finance-core` depends on nothing (beyond `std`, plus ADR-approved primitives).
- `finance-storage` may depend on `finance-core`; never the reverse.
- `finance-ui` may depend on `finance-core`; never on `finance-storage` directly — it goes through the application layer.
- Only `finance-app` may depend on everything.

A CI check will enforce this with `cargo tree` / `cargo deny` once the workspace exists.

## When to add a crate

Add one when it has a distinct **reason to change** and a boundary worth enforcing. Do not add one per module — folders are enough until a boundary carries a rule.
