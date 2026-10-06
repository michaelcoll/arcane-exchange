import type { CardImport } from '~/bindings/CardImport';
import type { CardImportStarted } from '~/bindings/CardImportStarted';
import type { CollectionParams } from '~/bindings/CollectionParams';
import type { CollectionStats } from '~/bindings/CollectionStats';
import type { PaginatedCollection } from '~/bindings/PaginatedCollection';
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';
import type { PriceHistoryParams } from '~/bindings/PriceHistoryParams';

export const useCollectionService = () => {
  const { apiCall } = useApi();

  const getCollection = (params?: MaybeRefOrGetter<CollectionParams>) =>
    useAsyncData(
      'collection',
      () => apiCall<PaginatedCollection>('/collection', { query: toValue(params) }),
      { lazy: true },
    );

  const importCards = (csv: string) =>
    apiCall<CardImportStarted>('/collection/import', {
      method: 'POST',
      body: csv,
      headers: { 'Content-Type': 'text/plain' },
    });

  const getCardImport = (id: string) => apiCall<CardImport>(`/collection/import/${id}`);

  const listCardImports = () => apiCall<CardImport[]>('/collection/import');

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

  return {
    getCollection,
    importCards,
    getCardImport,
    listCardImports,
    getCollectionStats,
    getPriceHistory,
  };
};
