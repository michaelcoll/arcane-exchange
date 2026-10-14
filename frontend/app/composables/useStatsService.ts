import type { Stats } from '~/bindings/Stats';

export const useStatsService = () => {
  const { publicApiCall } = useApi();

  // `GET /stats` is public: a signed-out visitor sends no bearer token.
  const getStats = () => publicApiCall<Stats>('/stats');

  return { getStats };
};
