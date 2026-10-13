import type { UserSuggestion } from '~/bindings/UserSuggestion';

export const useAutocompleteService = () => {
  const { publicApiCall } = useApi();

  // `GET /autocomplete/user` is public: a signed-out visitor sends no bearer token.
  const autocompleteUsers = (q: string) =>
    publicApiCall<UserSuggestion[]>('/autocomplete/user', { query: { q } });

  return { autocompleteUsers };
};
