import type { Stats } from '~/bindings/Stats';

export const useStatsService = () => {
  const config = useRuntimeConfig();

  // `GET /stats` is public: called without `useApi`, so a signed-out visitor sends no bearer token.
  const getStats = () => $fetch<Stats>(`${config.public.apiBase}/stats`);

  return { getStats };
};
