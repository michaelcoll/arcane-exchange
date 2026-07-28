import type { CardOffersParams } from '~/bindings/CardOffersParams';
import type { PaginatedCardOffers } from '~/bindings/PaginatedCardOffers';
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';
import type { PriceHistoryParams } from '~/bindings/PriceHistoryParams';

export const useCardsService = () => {
  const { apiCall } = useApi();

  const getCardInfo = () => apiCall('/card/card-info', { method: 'POST' });

  const getCardPriceHistory = (scryfallId: string, params?: PriceHistoryParams) =>
    apiCall<PriceHistoryEntry[]>(`/card/${scryfallId}/price-history`, { query: params });

  const getCardOffers = (params: CardOffersParams) =>
    apiCall<PaginatedCardOffers>('/card/offers', { query: params });

  return {
    getCardInfo,
    getCardPriceHistory,
    getCardOffers,
  };
};
