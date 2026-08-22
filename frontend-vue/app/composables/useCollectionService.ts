import type { CollectionParams } from '~/bindings/CollectionParams';
import type { CollectionStats } from '~/bindings/CollectionStats';
import type { Message } from '~/bindings/Message';
import type { PaginatedCollection } from '~/bindings/PaginatedCollection';
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';
import type { PriceHistoryParams } from '~/bindings/PriceHistoryParams';
import type { RarityFilters } from '~/bindings/RarityFilters';

export const useCollectionService = () => {
  const { apiCall } = useApi();

  const getCollection = (params?: MaybeRefOrGetter<CollectionParams>) =>
    useAsyncData(
      'collection',
      () => apiCall<PaginatedCollection>('/collection', { query: toValue(params) }),
      { lazy: true },
    );

  const importCards = (csv: string) =>
    apiCall<Message>('/collection/import', {
      method: 'POST',
      body: csv,
      headers: { 'Content-Type': 'text/plain' },
    });

  const getCollectionStats = () =>
    useAsyncData('collection-stats', () => apiCall<CollectionStats>('/collection/stats'), {
      lazy: true,
    });

  const getPriceHistory = (params: MaybeRefOrGetter<PriceHistoryParams>, key: string) =>
    useAsyncData(
      key,
      () => apiCall<PriceHistoryEntry[]>('/collection/price-history', { query: toValue(params) }),
      { lazy: true },
    );

  const getRarityFilters = () =>
    useAsyncData(
      'collection-rarity-filters',
      () => apiCall<RarityFilters>('/collection/visibility/rarities'),
      { lazy: true },
    );

  const setRarityFilter = (rarity: string, isOpen: boolean, keptCopies: number) =>
    apiCall<void>('/collection/visibility/rarities', {
      method: 'POST',
      body: { rarity, is_open: isOpen, kept_copies: keptCopies },
    });

  return {
    getCollection,
    importCards,
    getCollectionStats,
    getPriceHistory,
    getRarityFilters,
    setRarityFilter,
  };
};
