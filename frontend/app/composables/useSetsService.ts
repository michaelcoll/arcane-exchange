import type { SetInfo } from '~/bindings/SetInfo';

export const useSetsService = () => {
  const { apiCall } = useApi();

  const getSets = () => apiCall<SetInfo[]>('/sets');

  const getSet = (setCode: string) => apiCall<SetInfo>(`/sets/${setCode}`);

  return {
    getSets,
    getSet,
  };
};
