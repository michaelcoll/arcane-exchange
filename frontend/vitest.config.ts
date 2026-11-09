import { defineConfig } from 'vitest/config';
import { defineVitestProject } from '@nuxt/test-utils/config';

export default defineConfig({
  test: {
    reporters: ['dot'],
    onConsoleLog(log, type) {
      if (type === 'stdout') return false;
    },
    projects: [
      {
        test: {
          name: 'unit',
          include: ['app/**/*.{test,spec}.ts', 'server/**/*.{test,spec}.ts'],
          environment: 'node',
        },
      },
      {
        test: {
          name: 'e2e',
          include: ['test/e2e/*.{test,spec}.ts'],
          environment: 'node',
        },
      },
      await defineVitestProject({
        test: {
          name: 'nuxt',
          include: ['test/nuxt/*.{test,spec}.ts'],
          environment: 'nuxt',
          // The Nuxt app boots the Clerk plugin, which fetches its scripts from Clerk's CDN when a
          // publishable key is set. A test file that ends before the fetch does aborts it, and
          // @clerk/shared rethrows that failure in a detached promise: Vitest then reports an
          // unhandled error and exits non-zero. happy-dom loads no external script at all here.
          environmentOptions: {
            happyDom: {
              settings: {
                disableJavaScriptFileLoading: true,
                handleDisabledFileLoadingAsSuccess: true,
              },
            },
          },
        },
      }),
    ],
  },
});
