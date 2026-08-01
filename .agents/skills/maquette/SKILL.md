---
name: maquette
description: Explains how to consult the "The Arcane Exchange" UI mockup (maquette/ folder, not tracked by git) via the browser skill, to use it as a design/UX reference before implementing a screen in frontend-vue. Use when the user mentions the mockup, "maquette", "The Arcane Exchange", or asks to check/compare the design before coding a screen.
---

You are consulting this project's UI mockup to use it as a visual/UX reference before implementing a real feature.

## What the mockup is

- A standalone UI prototype ("The Arcane Exchange"), React 18 + Babel standalone loaded from CDN, **no build step**.
  This is not code to port as-is: the real stack is `frontend-vue` (Nuxt 4 / Vue 3 / Tailwind), not React.
- Lives in the `maquette/` folder at the repo root. It is in `.gitignore`: never committed, never referenced in a PR,
  don't try to version it.
- Serves as the visual source of truth behind `../../../.agents/design-system.instructions.md` (color tokens, spacing,
  components) and `../../../.agents/frontend.instructions.md` — those files partly document what's visible in the
  mockup. Check them for the translation into Tailwind classes, but ignore the Angular code sample they contain: the
  project's real stack is Vue, not Angular.

## How to consult it

- Main page: `maquette/The Arcane Exchange.html`, served by the http-server web server run using the mise command
  `mise maquette`, at: `http://localhost:4000/The%20Arcane%20Exchange.html`
- To explore it: use the **browser** skill (`playwright-cli`) — run `playwright-cli open` first, then
  `playwright-cli goto`
  to navigate, `playwright-cli snapshot` to read the page structure, `playwright-cli screenshot --filename=<name>.png`
  for a visual capture, `playwright-cli click` to interact with elements. Save all screenshots to `.playwright/`
  at the repo root (project rule, see AGENTS.md).
- `maquette/Design System.html` is a separate page that visually documents the design tokens (colors, components)
  — useful for checking a specific style without navigating the whole prototype.

## Folder structure

- `app.jsx` — app shell: client-side routing (state + `localStorage`, no real per-screen URL), dark/light theme toggle
  (`data-theme`), dev tweaks panel.
- `screen_home.jsx`, `screen_collection.jsx`, `screen_trade.jsx`, `screen_find.jsx`, `screen_prefs.jsx` — one file per
  screen.
- `components.jsx` — shared UI components (icons, etc.).
- `trade_store.jsx` — mocked state and data for the trade flow.
- `error_states.jsx` + the "Simuler une panne" toggle in the tweaks panel — for viewing API error states.
- `tweaks-panel.jsx` — dev panel (density, accent color, error simulation), not a product component.
- `styles.css` — global stylesheet (CSS tokens, dark/light themes).

## Navigation in the app

4 screens in the nav (desktop top nav + mobile bottom nav): **Collection**, **Échanges** (trade), **Rechercher**
(find), **Profil** (prefs). The logo top-left goes back to the `home` screen (landing, outside the main nav).

## Working method

1. Before implementing a screen or component that already exists in the mockup, consult it with the **browser** skill
   (`playwright-cli`) — `snapshot` for structure, `screenshot` for visuals, `click` for interactions — and note layout,
   visual hierarchy, and behaviors (hover, transitions, empty/error states) rather than guessing.
2. Translate into Vue/Tailwind following the conventions already in place in `frontend-vue`, not by copying the mockup's
   JSX/CSS. Use `../../../.agents/design-system.instructions.md` for tokens (colors, spacing, radius, button/badge
   variants).
3. If the mockup diverges from `../../../.agents/design-system.instructions.md`, flag it to the user rather than
   deciding unilaterally — the mockup may have evolved since the instructions were written.
