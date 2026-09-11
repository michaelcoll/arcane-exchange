import type { CardOffersParams } from '~/bindings/CardOffersParams';
import type { CardPriceHistoryParams } from '~/bindings/CardPriceHistoryParams';
import type { PaginatedCardOffers } from '~/bindings/PaginatedCardOffers';
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';

export const useCardsService = () => {
  const { apiCall } = useApi();

  const getCardInfo = () => apiCall('/card/card-info', { method: 'POST' });

  // `foil` is required: the catalog no longer knows a card's finish, so the caller must always
  // say which price series it wants.
  const getCardPriceHistory = (scryfallId: string, params: CardPriceHistoryParams) =>
    apiCall<PriceHistoryEntry[]>(`/card/${scryfallId}/price-history`, { query: params });

  const getCardOffers = (params: CardOffersParams) =>
    apiCall<PaginatedCardOffers>('/card/offers', { query: params });

  return {
    getCardInfo,
    getCardPriceHistory,
    getCardOffers,
  };
};
