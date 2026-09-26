# Frontend Development Guide (Nuxt/Vue/Tailwind)

## Design System & Styling (CRITICAL)

- **Absolute Rule**: Tailwind CSS (v3, via `@nuxtjs/tailwindcss` — config lives in `nuxt.config.ts`, not a separate
  `tailwind.config.js`). No `<style>` blocks in `.vue` components for ordinary business styling (compose with
  Tailwind classes in the template).
- **`<style scoped>` exceptions**: only for effects Tailwind utilities can't express cleanly — complex `@keyframes`,
  layered gradients/pseudo-elements (e.g. the foil card effect in `app/components/MtgCard.vue`).
- **Tokens**: colors/spacing/radius/shadow/typography are CSS custom properties defined in
  `app/assets/css/main.css` (`:root`). Accents are exposed as semantic Tailwind colors (`primary`, `secondary`,
  `rarity` — e.g. `text-primary`, `bg-secondary/10`, `text-rarity-rare`) that follow the theme without `dark:` pairs.
  Other tokens have no Tailwind utility (no `bg-surface`): apply them via the arbitrary-value syntax
  (`bg-[var(--surface)]`, `text-[var(--ink-2)]`, `rounded-[var(--r-lg)]`). Tailwind's default palette is only for
  neutrals (`slate`/`zinc`) and semantic red/green (`red`/`emerald`); `cyan` / `violet` classes are rejected by ESLint.
  - **Key rules (see `design-system.instructions.md` for the full token reference and component inventory):**
    - **Palette**: dark, glass, neon theme (`--bg`/`--surface` as base, `primary` and `secondary` picked by role).
    - **No borders**: prefer background-color shifts and spacing over `border` to define boundaries.
    - **Typography**: `font-display` (Space Grotesk) for titles, `font-mono` (JetBrains Mono) for
      numbers/prices/labels, `font-sans` (Hanken Grotesk) for everything else.
    - **Elevation**: tonal shifts between surface levels, not drop shadows.
    - **Theme toggle**: driven by the `.dark` class on `<html>` (Tailwind `darkMode: 'class'`), not a
      `data-theme` attribute.
    - **Text color**: always `--ink` (never pure white on dark surfaces).
    - **Interactions**: card hover = subtle scale + surface-tint background shift.
- **Icons**: only `<Icon name="lucide:…" :size="…" />` (`@nuxt/icon`). No hand-rolled SVG icon components or
  inline icon paths.

**For the full token reference and component inventory, refer to
[design-system.instructions.md](design-system.instructions.md).**
