# Design

**Status:** placeholder — visual direction to be set after requirements
**Last updated:** 2026-11-12

## What this document will hold

- Visual language: type scale, spacing scale, colour tokens, elevation, motion.
- Screen inventory and layout principles.
- Rune-specific implementation guidance (theming, motion, virtualisation, accessibility).
- Empty states, error states, and destructive-action patterns.

## Principles we already know

1. **Numbers first.** The register is the product. Everything else is a way to get back to it.
2. **Density with air.** A finance table needs to show a lot per row, but rows must remain scannable. Fixed-height rows, right-aligned amounts, tabular figures.
3. **Keyboard is a first-class input.** Enter a transaction without touching the mouse (FR-018).
4. **Never hide a discrepancy.** If numbers disagree, say so in the UI. Silence is how trust is lost.
5. **Motion explains, never decorates.** A spring communicates where something came from; if it does not, remove it.
6. **Colour is never the only signal** (NFR-044). Income/expense, cleared/uncleared and error states all carry a second cue.

## Rune notes

- Rune is a native GPU-rendered framework with its own layout, text and spring system; components come from `rune-kit`.
- Theme through tokens, not literals: no hard-coded colours or spacing in view code.
- Use Rune's virtualised rows for the register; build only mounted rows.
- Keep the UI layer free of business logic — views read view models produced by the application layer.
- Verify visual work headlessly: render to PNG, inspect the element tree, then look at the image.
- Respect Rune's frame rules: no file I/O during frames, no idle repaints (NFR-016).

## Still to decide

- Theme direction (light-first, dark-first, both from day one).
- Charting approach: Rune-drawn charts versus a small charting crate.
- Date picker and amount input components (build or adapt from `rune-kit`).
- Icon set and licensing.
- Onboarding: how much explanation a first-run user gets about accounts and categories.
