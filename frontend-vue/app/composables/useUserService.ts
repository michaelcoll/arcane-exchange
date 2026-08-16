import type { CollectionVisibility } from '~/bindings/CollectionVisibility';
import type { TradeBindersResponse } from '~/bindings/TradeBindersResponse';
import type { VisibilityResponse } from '~/bindings/VisibilityResponse';

export const useUserService = () => {
  const { apiCall } = useApi();

  const register = () => apiCall(`/user/register`, { method: 'POST' });

  const getVisibility = () => apiCall<VisibilityResponse>('/user/visibility');

  const setVisibility = (visibility: CollectionVisibility) =>
    apiCall<void>('/user/visibility', { method: 'PUT', body: { visibility } });

  const getTradeBinders = () => apiCall<TradeBindersResponse>('/user/trade-binders');

  const addTradeBinder = (binderName: string) =>
    apiCall<void>('/user/trade-binders', { method: 'POST', body: { binder_name: binderName } });

  const removeTradeBinder = (binderName: string) =>
    apiCall<void>(`/user/trade-binders/${encodeURIComponent(binderName)}`, { method: 'DELETE' });

  return {
    register,
    getVisibility,
    setVisibility,
    getTradeBinders,
    addTradeBinder,
    removeTradeBinder,
  };
};
