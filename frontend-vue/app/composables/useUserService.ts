import type { CollectionVisibility } from '~/bindings/CollectionVisibility';
import type { VisibilityResponse } from '~/bindings/VisibilityResponse';

export const useUserService = () => {
  const { apiCall } = useApi();

  const register = () => apiCall(`/user/register`, { method: 'POST' });

  const getVisibility = () => apiCall<VisibilityResponse>('/user/visibility');

  const setVisibility = (visibility: CollectionVisibility) =>
    apiCall<void>('/user/visibility', { method: 'PUT', body: { visibility } });

  return { register, getVisibility, setVisibility };
};
