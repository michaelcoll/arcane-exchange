import type { UserSuggestion } from '~/bindings/UserSuggestion';

export const useAutocompleteService = () => {
  const config = useRuntimeConfig();
  const base = config.public.apiBase;

  const autocompleteUsers = (q: string) =>
    $fetch<UserSuggestion[]>(`${base}/autocomplete/user`, { query: { q } });

  return { autocompleteUsers };
};
