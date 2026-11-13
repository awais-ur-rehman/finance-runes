# finance-core

**Status:** in progress — money, accounts, categories, transactions, budgets and
remembered rules are implemented; insights are not.

The domain. Everything that must be true about money, independent of how it is stored or displayed.

## Owns

- `Money` and `Currency` — integer minor units, ISO 4217 exponent (ADR-0003).
- `Account`, `AccountKind` (asset / liability).
- `Transaction`, `Posting`, `Transfer`, `Split` and their invariants (ADR-0005).
- `Category` hierarchy.
- `Budget`, periods, rollover rules.
- `Rule` — what the user taught the app about their own money, keyed on a merchant name or a transfer pattern (FR-033, ADR-0007).
- Derived calculations: balances as of a date, cash flow, net worth, budget status.
- Report queries, expressed as pure functions over a ledger.

## Must not

- Read or write files, databases or the network.
- Read the system clock or generate random IDs — take a `Clock` and an `IdGenerator` from the caller.
- Depend on Rune, `finance-storage`, `finance-ui` or any async runtime.
- Format for display beyond a debug representation. Locale formatting belongs to the UI edge.

## Why it is separate

This crate is where a bug costs trust. It must be testable in milliseconds, with no window and no disk, and it must survive a UI framework rewrite. See [ADR-0002](../../docs/architecture/decisions/0002-rust-and-rune.md).

## Testing

Unit tests plus property tests for every arithmetic invariant. See [testing strategy](../../docs/engineering/testing-strategy.md).

## Entry point (planned)

```rust
pub use money::{Currency, Money};
pub use ledger::{Account, AccountId, Posting, Transaction, TransactionId};
```

`mod` and `pub use` lists stay one item per line and sorted, per [coding standards](../../docs/engineering/coding-standards.md).
