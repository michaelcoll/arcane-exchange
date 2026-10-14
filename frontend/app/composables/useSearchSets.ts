import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import type { SetInfo } from '~/bindings/SetInfo';

/**
 * The sets of the cards a search can return (see `searchSetsScope`), refetched when the scope
 * changes value. No scope, no call and no set.
 */
export const useSearchSets = (scope: MaybeRefOrGetter<SearchSetsParams | null>) => {
  const { getSearchSets } = useSearchService();

  return useAsyncData<SetInfo[]>(
    'search-sets',
    async () => {
      const query = toValue(scope);
      return query ? getSearchSets(query) : [];
    },
    {
      lazy: true,
      default: () => [],
      // Compared by value: a scope rebuilt identical (e.g. the player's text filter changes)
      // does not refetch.
      watch: [() => JSON.stringify(toValue(scope))],
    },
  );
};
