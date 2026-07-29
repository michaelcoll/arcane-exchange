import type { PaginatedCollection } from '~/bindings/PaginatedCollection';
import type { SearchParams } from '~/bindings/SearchParams';

export const useSearchService = () => {
  const { apiCall } = useApi();

  const getSearch = (params?: MaybeRefOrGetter<SearchParams>) =>
    useAsyncData(
      'search',
      () => apiCall<PaginatedCollection>('/search/card', { query: toValue(params) }),
      { lazy: true },
    );

  return {
    getSearch,
  };
};
