// @ts-check
import withNuxt from './.nuxt/eslint.config.mjs';
import prettierConfig from 'eslint-config-prettier';

// Accents go through the semantic `primary` / `secondary` colors (backed by the main.css tokens),
// never Tailwind's default `cyan` / `violet` palette (e.g. `text-cyan-600`, `dark:bg-violet-400/10`).
const PALETTE_ACCENT_CLASS = String.raw`/(^|[\s:])[a-z][a-z-]*-(cyan|violet)-/`;
const PALETTE_ACCENT_MESSAGE =
  'Use the semantic `primary` / `secondary` Tailwind colors instead of the default cyan / violet palette.';
const paletteAccentRestrictions = ['Literal', 'TemplateElement', 'VLiteral'].map((node) => ({
  selector: `${node}[${node === 'TemplateElement' ? 'value.raw' : 'value'}=${PALETTE_ACCENT_CLASS}]`,
  message: PALETTE_ACCENT_MESSAGE,
}));

export default withNuxt(prettierConfig, {
  rules: {
    // Props are declared with TypeScript (`defineProps<{ foo?: T }>()`): an optional prop is
    // `undefined` when omitted, which is the intended default. The rule, written for runtime prop
    // declarations, would force a meaningless `withDefaults({ foo: undefined })` on each of them.
    'vue/require-default-prop': 'off',
  },
}).append({
  files: ['app/**/*.{ts,vue}'],
  rules: {
    'no-restricted-syntax': ['error', ...paletteAccentRestrictions],
    'vue/no-restricted-syntax': ['error', ...paletteAccentRestrictions],
  },
});
