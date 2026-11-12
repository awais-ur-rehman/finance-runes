# finance-ui

**Status:** planned — no code yet

Rune views, theming, input handling and locale-aware formatting. Deliberately thin.

## Owns

- Screens: accounts, register, transaction editor, import wizard, budgets, reports, settings.
- View models produced from domain data — the only place display strings are built.
- Theme tokens (colour, type, spacing, motion) and light/dark variants.
- Input: keyboard-first flows, shortcuts, command palette.
- Formatting: dates, numbers, currency, pluralisation, per locale (NFR-050).

## Must not

- Own business rules or compute a balance. Ask the application layer.
- Touch the filesystem or the database.
- Perform I/O on the UI thread during a frame (a Rune rule; on Windows each file call can block on antivirus).
- Hard-code user-visible strings — they are externalised for i18n (NFR-052).

## Rune specifics

- Pin Rune to a `dev` rev; upgrade in a dedicated PR with headless render checks.
- Use `rune-kit` components where they fit; build only what is missing.
- Virtualise the register (`virtual_rows`) and memoise rows that rarely change.
- Start slow work after the first frame and land the result with `tx.instant()` so launch stays still.
- Bound every cache in entries and bytes.

## Verification

Headless renders, element-tree assertions, golden images at 100% and 150% scale, keyboard-only scripts, and frame-time traces. See [testing strategy](../../docs/engineering/testing-strategy.md) and [design](../../docs/design/README.md).
