import type { PaginatedCollection } from '~/bindings/PaginatedCollection';
import type { SearchParams } from '~/bindings/SearchParams';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import type { SetInfo } from '~/bindings/SetInfo';

export const useSearchService = () => {
  const { apiCall } = useApi();

  const getSearch = (params?: MaybeRefOrGetter<SearchParams>) =>
    useAsyncData(
      'search',
      () => apiCall<PaginatedCollection>('/search/card', { query: toValue(params) }),
      { lazy: true },
    );

  /**
   * The sets of the cards a search can return (see `searchSetsScope`), refetched when the scope
   * changes. No scope, no set.
   */
  const getSearchSets = (scope: MaybeRefOrGetter<SearchSetsParams | null>) =>
    useAsyncData(
      'search-sets',
      async () => {
        const query = toValue(scope);
        return query ? apiCall<SetInfo[]>('/search/card/sets', { query }) : [];
      },
      {
        lazy: true,
        default: () => [],
        // Compared by value: a scope rebuilt identical (e.g. the player's text filter changes)
        // does not refetch.
        watch: [() => JSON.stringify(toValue(scope))],
      },
    );

  return {
    getSearch,
    getSearchSets,
  };
};
