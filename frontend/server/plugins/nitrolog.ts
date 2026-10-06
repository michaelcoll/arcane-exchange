import * as Sentry from '@sentry/nuxt';

export default defineNitroPlugin((nitro) => {
  nitro.hooks.hook('error', (error, { event }) => {
    console.error(`${event?.path} Application error:`, error);
    Sentry.captureException(error, {
      extra: { path: event?.path },
    });
  });
});
