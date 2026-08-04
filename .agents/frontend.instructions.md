# Frontend Development Guide (Nuxt/Vue/Tailwind)

## Design System & Styling (CRITICAL)

- **Absolute Rule**: Tailwind CSS (v3, via `@nuxtjs/tailwindcss` — config lives in `nuxt.config.ts`, not a separate
  `tailwind.config.js`). No `<style>` blocks in `.vue` components for ordinary business styling (compose with
  Tailwind classes in the template).
- **`<style scoped>` exceptions**: only for effects Tailwind utilities can't express cleanly — complex `@keyframes`,
  layered gradients/pseudo-elements (e.g. the foil card effect in `app/components/MtgCard.vue`).
- **Tokens**: colors/spacing/radius/shadow/typography are CSS custom properties defined in
  `app/assets/css/main.css` (`:root`), not Tailwind theme colors — there is no `bg-surface`/`text-on-surface`
  utility. Apply a token via Tailwind's arbitrary-value syntax (`bg-[var(--surface)]`,
  `text-[var(--ink-2)]`, `rounded-[var(--r-lg)]`), or fall back to Tailwind's default palette
  (`slate`/`zinc`/`cyan`/`violet`/`emerald`/`red` + `dark:` variants) as already used across
  `app/components/*.vue` when the design system doesn't prescribe an exact token.
  - **Key rules (see `design-system.instructions.md` for the full token reference and component inventory):**
    - **Palette**: dark, glass, neon cyan/violet theme (`--bg`/`--surface` as base, `--cyan`, `--violet`).
    - **No borders**: prefer background-color shifts and spacing over `border` to define boundaries.
    - **Typography**: `font-display` (Space Grotesk) for titles, `font-mono` (JetBrains Mono) for
      numbers/prices/labels, `font-sans` (Hanken Grotesk) for everything else.
    - **Elevation**: tonal shifts between surface levels, not drop shadows.
    - **Theme toggle**: driven by the `.dark` class on `<html>` (Tailwind `darkMode: 'class'`), not a
      `data-theme` attribute.
    - **Text color**: always `--ink` (never pure white on dark surfaces).
    - **Interactions**: card hover = subtle scale + surface-tint background shift.

**For the full token reference and component inventory, refer to
[design-system.instructions.md](design-system.instructions.md).**
