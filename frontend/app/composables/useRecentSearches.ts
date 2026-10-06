const MAX_RECENTS = 4;

export const CARD_RECENT_SEARCHES_KEY = 'tae_recent_searches';

export function getRecentSearches(key: string): string[] {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function saveRecentSearches(key: string, searches: string[]): void {
  try {
    localStorage.setItem(key, JSON.stringify(searches.slice(0, MAX_RECENTS)));
  } catch {
    // localStorage désactivé — silently ignore
  }
}

export function addRecentSearch(key: string, recents: string[], term: string): string[] {
  if (!term.trim()) return recents;
  const trimmed = term.trim();
  const filtered = recents.filter((s) => s !== trimmed);
  const updated = [trimmed, ...filtered].slice(0, MAX_RECENTS);
  saveRecentSearches(key, updated);
  return updated;
}

export function useRecentSearches(key: string) {
  const recents = ref<string[]>(getRecentSearches(key));

  function addRecentSearchWrapper(term: string): void {
    recents.value = addRecentSearch(key, recents.value, term);
  }

  return {
    recents,
    addRecentSearch: addRecentSearchWrapper,
  };
}
