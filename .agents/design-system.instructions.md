# The Arcane Exchange — Design System

Hi-fi design system for a Magic: The Gathering card trading platform. Aesthetic: **dark, frosted glass, cyan / violet
neon**, shades derived in `oklch`.

**Architecture**: tokens (colors, spacing, radius, shadows, typography) are CSS custom properties defined in
`frontend-vue/app/assets/css/main.css` (`:root`) — never invent a token, use the real names. Visual composition is done
with **Tailwind classes directly in the Vue template** (utility-first), not with dedicated global CSS classes (`.btn`,
`.panel`, `.chip`…). That class system exists in the mockup (`maquette/styles.css`) but has no equivalent in
`frontend-vue` — that's not the goal, don't recreate it. To apply a token in a Tailwind class, use the arbitrary-value
syntax (`bg-[var(--surface-2)]`, `text-[var(--ink-2)]`, `border-[var(--line)]`,
`rounded-[var(--r-lg)]`…), or Tailwind's default palette (`slate`, `zinc`, `cyan`, `violet`, `emerald`, `red`…)
already used by existing components (`app/components/*.vue`) when the mockup doesn't prescribe an exact hue.

---

## 1. Foundations

### Colors — locked palette

| Role               | Token                     | Value     | Usage                                           |
| ------------------ | ------------------------- | --------- | ----------------------------------------------- |
| App background     | `--bg`                    | `#131313` | global background (+ radial cyan/violet aurora) |
| Surface            | `--surface`               | `#1c1b1b` | cards, panels, modals                           |
| Cyan (accent)      | `--cyan` / `--accent`     | `#00daf3` | action, interactive, rising values              |
| Violet (secondary) | `--violet` / `--accent-2` | `#cdbdff` | EDHREC, balances, reserved                      |

### Derived neutrals (oklch off the background)

`--surface-2` (raised) · `--surface-3` (hover) · `--ink` (primary text) · `--ink-2` (secondary) · `--ink-3`
(tertiary / labels) · `--ink-4` (faint)

### Derived accents (states from the two neon hues)

For each hue: `-soft` (light), `-dim` (dark), `-fill` / `-fill-2` (translucent tinted background), `-line`
(border), `-glow` (glowing shadow), `-ink` (readable text on `-fill`). E.g. cyan: `--cyan-soft`, `--cyan-dim`,
`--cyan-fill`, `--cyan-fill-2`, `--cyan-line`, `--cyan-glow`,
`--cyan-ink`. Same for violet.

### Semantic colors

- `--down` — value decrease (muted warm red) + `--down-fill`
- `--good` — discount / savings (green) + `--good-fill`
- rising values: reuse the cyan directly (`--cyan`)

### Lines & glass

- Borders: `--line` (9%), `--line-2` (14%, more pronounced), `--line-3` (5%, subtle)
- Glass: `--glass-blur: 12px`, `--glass-alpha: 0.603`, `--glass-bg` (surface/transparent blend)

### Radius & shadow

`--r-sm: 8px` · `--r-md: 12px` · `--r-lg: 16px` · `--r-xl: 22px`
`--shadow`: light inset + large soft drop shadow. `--maxw: 1180px`.

### Light theme

The theme is driven by the `.dark` class on `<html>` (Tailwind `darkMode: 'class'`, see `nuxt.config.ts`) — not a
`data-theme` attribute (that's the mockup's mechanism, not `frontend-vue`'s). In the absence of `.dark`,
`:root:not(.dark)` reassigns the same set of tokens for light mode (background `#eef0f2`, white surfaces, deepened neons
to stay readable, subtle black borders). Always code with `var(--*)` — never a hardcoded color — so both themes work.

### Tweakable accent

`--accent` drives the cyan; it's overridable (theme/accent picker in preferences). Roles point at it, so changing
`--accent` retints the whole app.

---

## 2. Typography

Three families (loaded via the `@nuxt/fonts` module, see `nuxt.config.ts`):

| CSS var          | Tailwind class | Family             | Usage                                                  |
| ---------------- | -------------- | ------------------ | ------------------------------------------------------ |
| `--font-display` | `font-display` | **Space Grotesk**  | titles, KPIs, brand                                    |
| `--font-body`    | `font-sans`    | **Hanken Grotesk** | body copy, UI (default font)                           |
| `--font-mono`    | `font-mono`    | **JetBrains Mono** | numbers, prices, labels, codes — enable `tabular-nums` |

The mockup's helpers (`.display`, `.h1`, `.mono`, `.label`, `.kpi`…) don't exist in `frontend-vue`: recompose the visual
effect with Tailwind classes (`text-*`, `font-*`, `tracking-*`, `uppercase`) on a case-by-case basis, not by recreating
them as global classes.

Body: `text-[15px] leading-normal tracking-[0.01em]` (antialiasing already handled globally in `main.css`).

---

## 3. Layout

No shared layout classes (`.app`, `.appbar`, `.row`/`.col`…) — each screen/component composes its layout with Tailwind
flex/grid directly (see `app/app.vue` for the current shell: sticky header + `backdrop-blur`, desktop nav up top /
mobile nav fixed at the bottom). Reference points from the mockup to respect in this composition:

- Max page width `1180px` (`max-w-[1180px]`), narrow variant `680px` for content pages (forms, detail views).
- Mobile nav fixed at the bottom of the screen, desktop/mobile switch at the `md` breakpoint.
- Card grids in `auto-fill`/`minmax(...)`, `sm`/`lg` sizes depending on context (dense grid vs. featured display).
- The mockup drives gap/padding via a runtime tweak (`--d-gap`/`--d-pad`) — no need to reproduce that mechanism in
  `frontend-vue` without an explicit request; use fixed `gap-*`/`p-*` classes.

---

## 4. Surfaces

Visual patterns to compose in Tailwind (no dedicated `.panel`/`.card-surface`/`.inset` class in `frontend-vue`):

| Pattern             | Indicative Tailwind composition                                                                                                          | Role                                   |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| Frosted-glass panel | `bg-[var(--glass-bg)] backdrop-blur-[length:var(--glass-blur)] border border-[var(--line)] rounded-[var(--r-lg)] shadow-[var(--shadow)]` | main container (cards, panels, modals) |
| Flat surface        | `bg-[var(--surface)] rounded-[var(--r-md)]`                                                                                              | plain surface, no glass effect         |
| Inset area          | background darker than the parent surface + `rounded-[var(--r-md)]`                                                                      | recessed sub-area                      |
| Tinted accent box   | `bg-[var(--cyan-fill)]` / `bg-[var(--violet-fill)]`                                                                                      | accent highlight                       |

---

## 5. Components

Inventory of the mockup's UI patterns to reimplement as Vue components styled with Tailwind. The names below (`.btn`,
`.panel`…) are the ones from the mockup's CSS (`maquette/styles.css`) — useful for finding the reference style/behavior
to consult, **not classes to recreate as-is** in `frontend-vue`.

**Buttons** `.btn` + variants `.primary` (cyan), `.violet`, `.ghost`, `.danger`; sizes `.sm` / `.lg` / `.block`.

**Selection / filters**

- `.chip` (togglable pill, `.on` state, `.vio` variant)
- `.seg` — segmented control with an animated `.thumb` (`.on.cyan` / `.on.vio`) — already implemented in Tailwind in
  `app/components/SegToggle.vue`; use it as a composition reference for the other patterns in this list.
- `.set-pip` — set pip (Keyrune symbols), count badge `.set-ct`
- `.cbx` — multi-select set combobox (control, chips, popover, options)
- `.dual-range` — two-handle price-range slider

**Fields** `.field` (+ `.big`), glowing cyan focus; `.search-hero` for the search bar with a halo.

**MTG cards** `.mtg` — realistic monochrome frame (title bar, art, type bar, text box), quantity badge `.qty`, color
accents `.c-w/-u/-b/-r/-g/-m`, variants `.mini`, `.has-img` (real scan), `.foil` (holographic, scroll-driven),
`.clickable`. Grid cell `.card-cell` + deal indicators (`.deal-tag.good/.bad/.par`) — already implemented in
`app/components/MtgCard.vue` and `app/components/CardCell.vue` respectively.

**Official symbols**: mana (`.msym`, `@font-face` ManaSym, WUBRG badges) and set symbols (Keyrune, classes `ss`/
`ss-{code}`, loaded from the jsdelivr CDN in `nuxt.config.ts` — not self-hosted, unlike ManaSym).

**List rows** `.lrow` (+ `.locked`), `.pavatar` (player avatar, `.online` state — see
`app/components/PlayerAvatar.vue`), `.bar` (progress bar).

**Graphs**

- `.spark` — bar sparkline (see `app/components/Sparkline.vue`)
- `.graph` — simple SVG curve (line + fill + `.gtip` tooltip)
- `.valuebar` / `.egraph2` — envelope graph (low→trend band + average curve), `.is-compact` ⇄ `.is-detail` states, axes
  and grid, hover popover `.egtip` (date + Trend/Average/Low) — see `app/components/EnvelopeGraph.vue`. **This is the
  reference graph** for price evolution (collection and card detail).

**Trade / lifecycle**: `.statuspill` (tonal status pills), `.lifecycle` (stepper), `.stbanner` (tonal contextual
banner), `.balance` / `.bal-split` (trade value split), `.rating` (stars), `.reserved-flag`.

**Overlays**: `.overlay` (+ `.center-modal`), `.modal` (+ `.modal-card` for card detail), `.sheet` (mobile bottom
sheet).

**Notifications**: `.notif-wrap` / `.notif-badge` / `.notif-pop` / `.notif-item` (`.unread` state, tonal icons by type).

**Errors**: `.api-toast` (failed-action snackbar with retry), `.spin` (spinner).

**Preferences**: `.theme-grid` / `.theme-tile` (theme & accent picker).

---

## 6. Motion

Short, lively transitions (~.15–.3s, `cubic-bezier` curves with a slight overshoot) — reproduce with Tailwind utilities
(`transition-*`, `duration-*`, `ease-[cubic-bezier(...)]`) or, for complex named animations, component- local
`@keyframes`. Mockup animations to look up if needed: `pop` (modal), `slideup` (sheet), `fade` (overlay),
`toastIn`, `cbxIn`, `spin360`, `foilSlide`, `vbRangeIn` (`fade`, `pop`, `slideup` and `foilSlide` are already ported in
`main.css`). Everything must be neutralized under `prefers-reduced-motion: reduce`.

---

## 7. Golden Rules

1. **Always** go through the real `var(--*)` tokens; never a hardcoded color/font (otherwise the light theme breaks).
2. Two accent colors max: cyan (action/increase) + violet (secondary/balance). Red/green are reserved for semantic use
   (decrease/discount).
3. Numbers, prices, and labels in `--font-mono` (`font-mono`); titles in `--font-display` (`font-display`); everything
   else in `--font-body` (`font-sans`).
4. Glass surfaces (§4) for main containers, inset area for sub-zones — composed in Tailwind, not via global CSS classes.
5. No emojis, no aggressive gradients — the only tolerated gradient is the neon halo (`-glow`) and the background
   aurora.
6. Don't create global CSS classes like `.btn`/`.panel`/`.chip`: compose each Vue component in Tailwind, relying on the
   `var(--*)` tokens above (arbitrary values) and on already-written components (`app/components/*.vue`)
   as a style reference.
