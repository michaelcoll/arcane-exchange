import type { PaginatedCollection } from '~/bindings/PaginatedCollection';
import type { SearchParams } from '~/bindings/SearchParams';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import type { SetInfo } from '~/bindings/SetInfo';

export const useSearchService = () => {
  const { apiCall } = useApi();

  /** One page of a search: the page adapter of its card list (see `useCardList`). */
  const getSearch = (params: SearchParams) =>
    apiCall<PaginatedCollection>('/search/card', { query: params });

  /** The sets of the cards a search scope can return (see `useSearchSets`). */
  const getSearchSets = (scope: SearchSetsParams) =>
    apiCall<SetInfo[]>('/search/card/sets', { query: scope });

  return {
    getSearch,
    getSearchSets,
  };
};
