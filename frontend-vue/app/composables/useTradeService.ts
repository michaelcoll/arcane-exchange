import type { AddTradeCardRequest } from '~/bindings/AddTradeCardRequest';
import type { CreateTradeResponse } from '~/bindings/CreateTradeResponse';
import type { ListTradesParams } from '~/bindings/ListTradesParams';
import type { PaginatedTrades } from '~/bindings/PaginatedTrades';
import type { RateTradeRequest } from '~/bindings/RateTradeRequest';
import type { RemoveTradeCardRequest } from '~/bindings/RemoveTradeCardRequest';
import type { TradeDetail } from '~/bindings/TradeDetail';

export const useTradeService = () => {
  const { apiCall } = useApi();

  const getTrade = (tradeId: MaybeRefOrGetter<string>) =>
    useAsyncData(
      `trade-${toValue(tradeId)}`,
      () => apiCall<TradeDetail>(`/trades/${toValue(tradeId)}`),
      { lazy: true },
    );

  const listTrades = (params?: MaybeRefOrGetter<Partial<ListTradesParams>>) =>
    useAsyncData('trades', () => apiCall<PaginatedTrades>('/trades', { query: toValue(params) }), {
      lazy: true,
    });

  const createTrade = (respondentUsername: string) =>
    apiCall<CreateTradeResponse>('/trades', {
      method: 'POST',
      body: { respondent_username: respondentUsername },
    });

  const addCard = (tradeId: string, body: AddTradeCardRequest) =>
    apiCall<void>(`/trades/${tradeId}/cards`, { method: 'POST', body });

  const removeCard = (tradeId: string, body: RemoveTradeCardRequest) =>
    apiCall<void>(`/trades/${tradeId}/cards/remove`, { method: 'POST', body });

  const acceptTrade = (tradeId: string) =>
    apiCall<void>(`/trades/${tradeId}/accept`, { method: 'POST' });

  const abandonTrade = (tradeId: string) =>
    apiCall<void>(`/trades/${tradeId}/abandon`, { method: 'POST' });

  const confirmTrade = (tradeId: string) =>
    apiCall<void>(`/trades/${tradeId}/confirm`, { method: 'POST' });

  const rateTrade = (tradeId: string, rating: number) =>
    apiCall<void>(`/trades/${tradeId}/rate`, {
      method: 'POST',
      body: { rating } satisfies RateTradeRequest,
    });

  return {
    getTrade,
    listTrades,
    createTrade,
    addCard,
    removeCard,
    acceptTrade,
    abandonTrade,
    confirmTrade,
    rateTrade,
  };
};
