// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  ssr: false,
  devtools: { enabled: true },
  sourcemap: { client: 'hidden' },
  app: {
    head: {
      title: 'Arcane Exchange',
      link: [
        { rel: 'icon', type: 'image/png', href: '/favicon-96x96.png?v=20260719', sizes: '96x96' },
        {
          rel: 'icon',
          type: 'image/svg+xml',
          href: '/favicon.svg?v=20260719',
        },
        {
          rel: 'shortcut icon',
          href: '/favicon.ico?v=20260719',
        },
        {
          rel: 'apple-touch-icon',
          href: '/apple-touch-icon.png?v=20260719',
          sizes: '180x180',
        },
        {
          rel: 'manifest',
          href: '/site.webmanifest?v=20260719',
        },
        {
          rel: 'stylesheet',
          type: 'text/css',
          href: '//cdn.jsdelivr.net/npm/keyrune@latest/css/keyrune.css',
        },
      ],
      meta: [
        { name: 'application-name', content: 'Arcane Exchange' },
        { name: 'apple-mobile-web-app-title', content: 'Arcane Exchange' },
        { name: 'apple-mobile-web-app-capable', content: 'yes' },
        { name: 'apple-mobile-web-app-status-bar-style', content: 'black-translucent' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1, viewport-fit=cover' },
        {
          name: 'theme-color',
          content: '#0f1414',
          media: '(prefers-color-scheme: dark)',
        },
        {
          name: 'theme-color',
          content: '#edeff5',
          media: '(prefers-color-scheme: light)',
        },
      ],
    },
  },
  modules: [
    '@nuxtjs/tailwindcss',
    '@nuxt/icon',
    '@nuxt/eslint',
    '@sentry/nuxt/module',
    '@nuxt/fonts',
    '@clerk/nuxt',
    '@vueuse/nuxt',
    '@nuxt/test-utils/module',
  ],
  runtimeConfig: {
    public: {
      apiBase: '/api/v1',
    },
  },
  css: ['~/assets/css/main.css'],
  fonts: {
    families: [
      { name: 'Space Grotesk', provider: 'google', weights: [400, 500, 600, 700] },
      { name: 'Hanken Grotesk', provider: 'google', weights: [400, 500, 600, 700, 800] },
      { name: 'JetBrains Mono', provider: 'google', weights: [400, 500, 600, 700] },
    ],
  },
  tailwindcss: {
    config: {
      darkMode: 'class',
      content: [
        './app/components/**/*.{vue,js,ts}',
        './app/layouts/**/*.vue',
        './app/pages/**/*.vue',
        './app/app.vue',
        './app/**/*.{vue,js,ts}',
      ],
      theme: {
        extend: {
          fontFamily: {
            display: ['Space Grotesk', 'system-ui', 'sans-serif'],
            sans: ['Hanken Grotesk', 'system-ui', 'sans-serif'],
            mono: ['JetBrains Mono', 'ui-monospace', 'monospace'],
          },
          fontSize: {
            // micro labels (mono uppercase) — single non-standard token kept on purpose
            '2xs': ['0.625rem', { lineHeight: '1rem' }], // 10px
          },
        },
      },
    },
  },
  sentry: {
    org: process.env.SENTRY_ORG,
    project: process.env.SENTRY_PROJECT_FRONT,
    authToken: process.env.SENTRY_AUTH_TOKEN,
    telemetry: false,
    sourcemaps: {
      // As you're enabling client source maps, you probably want to delete them after they're uploaded to Sentry.
      // Set the appropriate glob pattern for your output folder - some glob examples below:
      filesToDeleteAfterUpload: [
        './**/*.map',
        '.*/**/public/**/*.map',
        '.output/**/public/**/*.map',
      ],
    },
  },
  nitro: {
    routeRules: {
      '/api/v1/**': {
        proxy: {
          to: `http://${process.env.NUXT_BACKEND_HOSTNAME}:8080/**`,
        },
      },
    },
  },
});
