# The Arcane Exchange — Design System

Hi-fi design system for a Magic: The Gathering card trading platform. Aesthetic: **dark, frosted glass, neon primary
(cyan) / secondary (violet)**, shades derived in `oklch`.

**Architecture**: tokens (colors, spacing, radius, shadows, typography) are CSS custom properties defined in
`frontend-vue/app/assets/css/main.css` (`:root`) — never invent a token, use the real names. Visual composition is done
with **Tailwind classes directly in the Vue template** (utility-first), not with dedicated global CSS classes (`.btn`,
`.panel`, `.chip`…). That class system exists in the mockup (`maquette/styles.css`) but has no equivalent in
`frontend-vue` — that's not the goal, don't recreate it. Accent colors go through the **semantic Tailwind colors**
`primary`, `secondary` and `rarity` (§1). Other tokens are applied with the arbitrary-value syntax
(`bg-[var(--surface-2)]`, `text-[var(--ink-2)]`, `border-[var(--line)]`, `rounded-[var(--r-lg)]`…). Tailwind's default
palette is only for neutrals (`slate`, `zinc`) and the semantic red/green (`red`, `emerald`) — **never `cyan` /
`violet`**: an ESLint rule fails `mise run lint-frontend` on any `*-cyan-*` / `*-violet-*` class in `frontend-vue/app`.

---

## 1. Foundations

### Colors — locked palette

| Role           | Token         | Dark      | Light                      | Usage                                               |
| -------------- | ------------- | --------- | -------------------------- | --------------------------------------------------- |
| App background | `--bg`        | `#131313` | `#eef0f2`                  | global background (+ radial primary/secondary aura) |
| Surface        | `--surface`   | `#1c1b1b` | `#ffffff`                  | cards, panels, modals                               |
| Primary        | `--primary`   | `#00daf3` | `oklch(0.488 0.084 209.4)` | see roles below                                     |
| Secondary      | `--secondary` | `#cdbdff` | `oklch(0.535 0.16 295)`    | see roles below                                     |

Colors are named by **role**, not by hue:

- **primary** — my actions, rising values, a trade's progress.
- **secondary** — what concerns another player or the exchange (reserved card, cash delta, player mode, partner,
  « Je reçois »), plus secondary data series.

Two accents max: never introduce a third hue, and never pick an accent by its look — pick it by its role.

### Derived neutrals (oklch off the background)

`--surface-2` (raised) · `--surface-3` (hover) · `--ink` (primary text) · `--ink-2` (secondary) · `--ink-3`
(tertiary / labels) · `--ink-4` (faint)

### Role variants

For `primary` and `secondary`: `-soft`, `-dim`, `-ink` (readable text on `-fill`) are explicit values per theme;
`-fill` / `-fill-2` (translucent tinted background), `-line` (border), `-glow` (glowing shadow) are translucent
`color-mix` derivatives of the base color. E.g. `--primary-soft`, `--primary-fill`, `--secondary-ink`.
`--on-primary` / `--on-secondary` are the text colors to put on a solid `primary` / `secondary` background.

Rarity (MTG convention): `--rarity-common`, `--rarity-uncommon`, `--rarity-rare`, `--rarity-mythic`,
`--rarity-special`, explicit per theme.

### Tailwind semantic colors

Declared in `nuxt.config.ts` (`theme.extend.colors`), backed by the tokens above — they follow the light/dark theme on
their own, so **no `dark:` pair** is needed:

| Color       | Classes                                                                                         |
| ----------- | ----------------------------------------------------------------------------------------------- |
| `primary`   | `text-primary`, `bg-primary-soft`, `text-primary-ink`, `bg-primary-fill`, `border-primary-line` |
| `secondary` | same keys: `DEFAULT`, `soft`, `dim`, `fill`, `fill-2`, `line`, `glow`, `ink`                    |
| `rarity`    | `text-rarity-common`, `-uncommon`, `-rare`, `-mythic`, `-special`                               |

The opacity modifier works (`bg-primary/10`, `border-secondary/40`) — there is no numeric scale (`primary-500`).
Usual mappings: text → `text-primary` / `text-secondary`; text on a tinted pill → `text-primary-ink`; solid button →
`bg-primary hover:bg-primary-soft text-[var(--on-primary)]`; tinted box → `border-primary/30 bg-primary/10`. The
`fill` / `line` / `glow` keys are already translucent: don't stack an opacity modifier on them. `soft` / `dim` are
lightness steps tuned per theme for contrast, not a fixed "lighter / darker" order — in light mode `soft` is darker
than the base (so `hover:bg-primary-soft` darkens a button in light and lightens it in dark).

### Contrast

Every role color that carries text (`primary`, `secondary`, their `-soft` / `-dim` / `-ink`, and the five
`--rarity-*`) reaches **WCAG AA 4.5:1** on `--bg`, `--surface` and `--surface-2`, in light **and** dark. Changing one
of these values means re-checking the ratio on the three backgrounds, in both themes. These values are the reference
for iOS.

### Semantic colors

- `--down` — value decrease (muted warm red) + `--down-fill`
- `--good` — discount / savings (green) + `--good-fill`
- rising values: `primary`

### Lines & glass

- Borders: `--line` (9%), `--line-2` (14%, more pronounced), `--line-3` (5%, subtle)
- Glass: `--glass-blur: 12px`, `--glass-alpha: 0.603`, `--glass-bg` (surface/transparent blend)

### Radius & shadow

`--r-sm: 8px` · `--r-md: 12px` · `--r-lg: 16px` · `--r-xl: 22px`
`--shadow`: light inset + large soft drop shadow. `--maxw: 1180px`.

### Light theme

The theme is driven by the `.dark` class on `<html>` (Tailwind `darkMode: 'class'`, see `nuxt.config.ts`) — not a
`data-theme` attribute (that's the mockup's mechanism, not `frontend-vue`'s). In the absence of `.dark`,
`:root:not(.dark)` reassigns the same set of tokens for light mode (background `#eef0f2`, white surfaces, deepened
role colors to stay readable, subtle black borders). Always code with the tokens or the semantic colors — never a
hardcoded color — so both themes work. There is no user-tweakable accent: `--primary` is a fixed token.

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
| Tinted accent box   | `border-primary/30 bg-primary/10` / `border-secondary/30 bg-secondary/10`                                                                | accent highlight                       |

---

## 5. Components

Inventory of the mockup's UI patterns to reimplement as Vue components styled with Tailwind. The names below (`.btn`,
`.panel`…) are the ones from the mockup's CSS (`maquette/styles.css`) — useful for finding the reference style/behavior
to consult, **not classes to recreate as-is** in `frontend-vue`.

**Buttons** `.btn` + variants `.primary`, `.violet` (→ `secondary`), `.ghost`, `.danger`; sizes `.sm` / `.lg` /
`.block`.

**Selection / filters**

- `.chip` (togglable pill, `.on` state, `.vio` variant → `secondary`)
- `.seg` — segmented control with an animated `.thumb` (`.on.cyan` / `.on.vio` in the mockup; `tone: 'primary'` /
  `'secondary'` in `frontend-vue`) — already implemented in Tailwind in `app/components/SegToggle.vue`; use it as a
  composition reference for the other patterns in this list.
- `.set-pip` — set pip (Keyrune symbols), count badge `.set-ct`
- `.cbx` — multi-select set combobox (control, chips, popover, options)
- `.dual-range` — two-handle price-range slider

**Fields** `.field` (+ `.big`), glowing primary focus; `.search-hero` for the search bar with a halo.

**MTG cards** `.mtg` — the mockup's monochrome frame (title bar, art, type bar, text box) is **not** reproduced: the app
always shows the stored card image, or the generic card back `public/card-back.webp` for a card without one (pending,
or failed to load). Quantity badge `.qty`, variants `.mini`, `.foil` (holographic, scroll-driven), `.clickable`, plus
a flip button to the back of a double-faced card (`flippable`, detail modal only). Grid cell `.card-cell` + deal indicators (`.deal-tag.good/.bad/.par`) — already implemented in
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

---

## 6. Motion

Short, lively transitions (~.15–.3s, `cubic-bezier` curves with a slight overshoot) — reproduce with Tailwind utilities
(`transition-*`, `duration-*`, `ease-[cubic-bezier(...)]`) or, for complex named animations, component- local
`@keyframes`. Mockup animations to look up if needed: `pop` (modal), `slideup` (sheet), `fade` (overlay),
`toastIn`, `cbxIn`, `spin360`, `foilSlide`, `vbRangeIn` (`fade`, `pop`, `slideup` and `foilSlide` are already ported in
`main.css`). Everything must be neutralized under `prefers-reduced-motion: reduce`.

---

## 7. Golden Rules

1. **Always** go through the real `var(--*)` tokens or the semantic Tailwind colors; never a hardcoded color/font
   (otherwise the light theme breaks), never the default `cyan` / `violet` palette.
2. Two accent colors max, chosen by role: `primary` (my actions, increase, trade progress) + `secondary` (other player,
   exchange, secondary series). Red/green are reserved for semantic use (decrease/discount).
3. Numbers, prices, and labels in `--font-mono` (`font-mono`); titles in `--font-display` (`font-display`); everything
   else in `--font-body` (`font-sans`).
4. Glass surfaces (§4) for main containers, inset area for sub-zones — composed in Tailwind, not via global CSS classes.
5. No emojis, no aggressive gradients — the only tolerated gradient is the neon halo (`-glow`) and the background
   aurora.
6. Don't create global CSS classes like `.btn`/`.panel`/`.chip`: compose each Vue component in Tailwind, relying on the
   `var(--*)` tokens above (arbitrary values) and on already-written components (`app/components/*.vue`)
   as a style reference.
