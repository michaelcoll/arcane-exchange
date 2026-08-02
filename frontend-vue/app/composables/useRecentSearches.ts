const RECENT_SEARCHES_KEY = 'tae_recent_searches';
const MAX_RECENTS = 4;

export function getRecentSearches(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_SEARCHES_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function saveRecentSearches(searches: string[]): void {
  try {
    localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(searches.slice(0, MAX_RECENTS)));
  } catch {
    // localStorage désactivé — silently ignore
  }
}

export function addRecentSearch(recents: string[], term: string): string[] {
  if (!term.trim()) return recents;
  const trimmed = term.trim();
  const filtered = recents.filter((s) => s !== trimmed);
  const updated = [trimmed, ...filtered].slice(0, MAX_RECENTS);
  saveRecentSearches(updated);
  return updated;
}

export function useRecentSearches() {
  const recents = ref<string[]>(getRecentSearches());

  function addRecentSearchWrapper(term: string): void {
    recents.value = addRecentSearch(recents.value, term);
  }

  return {
    recents,
    addRecentSearch: addRecentSearchWrapper,
  };
}
