# ADR-0002 — Rust and Rune, with a thin UI layer

**Status:** accepted
**Date:** 2026-11-12
**Deciders:** project owner
**Supersedes:** —

## Context

The app is a single-user, local-first desktop finance manager. It must start fast, scroll tens of thousands of rows without hitching, run on Windows/macOS/Linux, and keep the user's data on disk. It must also be maintainable by one developer working with coding agents.

Options considered:

| Option | Pros | Cons |
| --- | --- | --- |
| **Rust + [Rune](https://github.com/anomalyco/rune)** | Native GPU rendering (wgpu/winit), no web runtime, 120 Hz motion, headless render + element-tree dumps so agents can verify UI work, MIT | Pre-release; API churn; small ecosystem; must pin a `dev` rev |
| Rust + Tauri + web UI | Mature UI ecosystem, easy styling | Webview memory and startup cost, two languages, layout/scroll performance ceiling for dense tables |
| Electron / web app | Fastest to build, huge ecosystem | Heaviest runtime, data-in-browser trust problem, poor fit for "local-first and fast" |
| Native per-platform (Swift/C#/GTK) | Best platform integration | Three codebases for one developer |

## Decision

Build the app in **Rust**, with the UI in **Rune** (pinned to a specific `dev` rev, recorded in the workspace manifest).

Keep the UI layer thin: `finance-ui` contains views, view models, theming and formatting only. All business rules live in `finance-core`, which has no knowledge of Rune. A Rune upgrade must never be able to change a balance.

## Consequences

**Positive** — one language across domain and UI; native performance; strong correctness guarantees where money is handled; headless UI verification suits agent-assisted development.

**Negative** — Rune is pre-release, so breaking changes are expected and upgrades are deliberate work. We accept that cost in exchange for the boundary: upgrades touch one crate.

**Neutral** — we must write our own components that other stacks get for free (virtualised tables, date pickers), using `rune-kit` where it fits.

## Follow-ups

- [ ] Record the pinned Rune rev in the workspace manifest when the first crate lands, and bump it in a dedicated PR with a render check.
- [ ] Add a headless render test to CI before any UI work merges.
- [ ] Keep a small set of golden renders per screen to catch visual regressions.
