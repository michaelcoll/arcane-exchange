---
name: maquette-ios
description: Explains how to consult the "Arcane Exchange — application iOS" UI mockup (maquette/ folder, not tracked by git) via the browser skill, to use it as a design/UX reference before implementing a screen in the iOS app. Use when the user mentions the iOS mockup, "maquette iOS", or asks to check/compare the iOS design before coding a screen.
---

You are consulting this project's iOS UI mockup to use it as a visual/UX reference before implementing a real feature
in the native iOS app.

## What the mockup is

- A standalone UI prototype ("Arcane Exchange — application iOS"), React 18 + Babel standalone loaded from CDN,
  **no build step**. This is not code to port as-is: the real stack for the iOS app is native Swift/SwiftUI
  (`ios-app/`), not React.
- Lives in the `maquette/` folder at the repo root. It is in `.gitignore`: never committed, never referenced in a PR,
  don't try to version it.
- Serves as the visual source of truth for iOS-specific layout, navigation and interactions — a native-feeling
  reinterpretation of the web app's design system (see `../../../.agents/design-system.instructions.md` for the
  underlying tokens: colors, spacing, components), adapted to iOS conventions (tab bar, nav bar, sheets, etc.).

## How to consult it

- Main page: `maquette/Arcane Exchange iOS.html`, served by the http-server web server run using the mise command
  `mise maquette`, at: `http://localhost:4000/Arcane%20Exchange%20iOS.html`
- To explore it: use the **browser** skill (`playwright-cli`) — run `playwright-cli open` first, then
  `playwright-cli goto`
  to navigate, `playwright-cli snapshot` to read the page structure, `playwright-cli screenshot --filename=<name>.png`
  for a visual capture, `playwright-cli click` to interact with elements. Save all screenshots to `.playwright-cli/`
  at the repo root (project rule, see AGENTS.md).
- Since it's a phone-shaped UI, prefer emulating a mobile viewport when opening/screenshotting the page (narrow width,
  e.g. ~390×844) so the layout renders as intended instead of stretched to desktop width.
- `maquette/Design System.html` documents the shared design tokens (colors, components) — useful for checking a
  specific style without navigating the whole prototype.

## Folder structure

- `ios-frame.jsx` — device chrome / app shell for the iOS mockup: status bar, tab bar, navigation stack simulation.
- `ios-app.jsx` — app entry point: routing between iOS screens (state-based, no real per-screen URL).
- `ios-data.jsx` — mocked data specific to the iOS screens.
- `ios-kit.jsx` — iOS-specific shared UI components/primitives (nav bars, list rows, sheets, etc.), the iOS
  counterpart to `components.jsx`.
- `ios-screens-a.jsx`, `ios-screens-b.jsx`, `ios-screens-c.jsx`, `ios-screens-d.jsx` — iOS screens, split across
  several files.
- `ios.css` — iOS-specific stylesheet, layered on top of the shared `styles.css` (CSS tokens, dark/light themes).
- `components.jsx`, `trade_store.jsx`, `tweaks-panel.jsx` — shared with the web mockup (icons, mocked trade state,
  dev tweaks panel); see the `maquette` skill for details on these.
- `assets/cards/` — card artwork images used by the mockup.

## Working method

1. Before implementing a screen or component that already exists in the iOS mockup, consult it with the **browser**
   skill (`playwright-cli`) — `snapshot` for structure, `screenshot` for visuals, `click` for interactions — and note
   layout, visual hierarchy, and behaviors (transitions, empty/error states, native iOS patterns like swipe/sheet)
   rather than guessing.
2. Translate into native Swift/SwiftUI following the conventions already in place in `ios-app/`, not by copying the
   mockup's JSX/CSS. Use `../../../.agents/design-system.instructions.md` for the underlying tokens (colors, spacing,
   radius) and adapt them to SwiftUI idioms.
3. If the mockup diverges from `../../../.agents/design-system.instructions.md` or from iOS platform conventions, flag
   it to the user rather than deciding unilaterally — the mockup may have evolved since the instructions were written.
