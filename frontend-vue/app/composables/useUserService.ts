import type { UserProfileResponse } from '~/bindings/UserProfileResponse';

export const useUserService = () => {
  const { apiCall } = useApi();

  const register = () => apiCall(`/user`, { method: 'POST' });

  const getUserProfile = (username: string) =>
    apiCall<UserProfileResponse>(`/user/${encodeURIComponent(username)}`);

  return {
    register,
    getUserProfile,
  };
};
