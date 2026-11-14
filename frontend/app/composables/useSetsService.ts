import type { SetInfo } from '~/bindings/SetInfo';

export const useSetsService = () => {
  const { apiCall } = useApi();

  const getSets = () => apiCall<SetInfo[]>('/sets');

  /**
   * The sets the player owns at least one card of, most recent first and sets without a date
   * last, as the API orders them: the source of the collection's set filter.
   */
  const getOwnedSets = () => apiCall<SetInfo[]>('/sets', { query: { owned: true } });

  const getSet = (setCode: string) => apiCall<SetInfo>(`/sets/${setCode}`);

  return {
    getSets,
    getOwnedSets,
    getSet,
  };
};
