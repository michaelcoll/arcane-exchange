import type { CollectionVisibility } from '~/bindings/CollectionVisibility';
import type { RarityFilters } from '~/bindings/RarityFilters';
import type { TradeBindersResponse } from '~/bindings/TradeBindersResponse';
import type { VisibilityResponse } from '~/bindings/VisibilityResponse';

/* Réglages de mise à l'échange : visibilité de collection, binders ouverts à l'échange et
 * filtres de rareté. */
export const useTradeSettingsService = () => {
  const { apiCall } = useApi();

  const getVisibility = () => apiCall<VisibilityResponse>('/collection/trade-settings/visibility');

  const setVisibility = (visibility: CollectionVisibility) =>
    apiCall<undefined>('/collection/trade-settings/visibility', {
      method: 'PUT',
      body: { visibility },
    });

  const getTradeBinders = () => apiCall<TradeBindersResponse>('/collection/trade-settings/binders');

  const addTradeBinder = (binderName: string) =>
    apiCall<undefined>('/collection/trade-settings/binders', {
      method: 'POST',
      body: { binder_name: binderName },
    });

  const removeTradeBinder = (binderName: string) =>
    apiCall<undefined>(`/collection/trade-settings/binders/${encodeURIComponent(binderName)}`, {
      method: 'DELETE',
    });

  const getRarityFilters = () =>
    useAsyncData(
      'collection-rarity-filters',
      () => apiCall<RarityFilters>('/collection/trade-settings/rarities'),
      { lazy: true },
    );

  const setRarityFilter = (rarity: string, isOpen: boolean, keptCopies: number) =>
    apiCall<undefined>('/collection/trade-settings/rarities', {
      method: 'POST',
      body: { rarity, is_open: isOpen, kept_copies: keptCopies },
    });

  return {
    getVisibility,
    setVisibility,
    getTradeBinders,
    addTradeBinder,
    removeTradeBinder,
    getRarityFilters,
    setRarityFilter,
  };
};
