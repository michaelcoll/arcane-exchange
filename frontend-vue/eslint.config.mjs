// @ts-check
import withNuxt from './.nuxt/eslint.config.mjs';
import prettierConfig from 'eslint-config-prettier';

export default withNuxt(prettierConfig, {
  rules: {
    // Props are declared with TypeScript (`defineProps<{ foo?: T }>()`): an optional prop is
    // `undefined` when omitted, which is the intended default. The rule, written for runtime prop
    // declarations, would force a meaningless `withDefaults({ foo: undefined })` on each of them.
    'vue/require-default-prop': 'off',
  },
});
