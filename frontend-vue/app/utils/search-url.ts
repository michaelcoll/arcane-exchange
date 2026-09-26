import type { LocationQuery } from 'vue-router';

export const SEARCH_MODES = ['name', 'decklist', 'player'] as const;
export type SearchMode = (typeof SEARCH_MODES)[number];

export const isSearchMode = (v: unknown): v is SearchMode => SEARCH_MODES.includes(v as SearchMode);

/**
 * The search as the URL of `/search` carries it, so that a reload replays it. Sort, rarity, set,
 * price and display size stay out of the URL.
 *
 * - `q`: the submitted card name (`name` mode);
 * - `player`: the browsed player (`player` mode), and `filter` the text filter on their collection.
 *
 * The decklist text is too long for the URL: `mode=decklist` only says to read it back from
 * sessionStorage (see `loadDecklist`).
 */
export interface SearchUrlState {
  mode: SearchMode;
  q?: string;
  player?: string;
  filter?: string;
}

const nonBlankString = (v: unknown): string | undefined =>
  typeof v === 'string' && v.trim() ? v : undefined;

/**
 * Reads the search from the URL. `player` without a `mode` (the links from the home page and a
 * trade) opens that player's collection; `q` only counts in `name` mode, `filter` only with a
 * player.
 */
export const parseSearchQuery = (query: LocationQuery): SearchUrlState => {
  const mode = isSearchMode(query.mode) ? query.mode : undefined;
  const player = nonBlankString(query.player);

  if (player && (mode === 'player' || !mode)) {
    const filter = nonBlankString(query.filter);
    return { mode: 'player', player, ...(filter && { filter }) };
  }
  if (!mode || mode === 'name') {
    const q = nonBlankString(query.q);
    return { mode: 'name', ...(q && { q }) };
  }
  return { mode };
};

/** Builds the URL query of a search, the inverse of `parseSearchQuery`. */
export const toSearchQuery = (state: SearchUrlState): Record<string, string> => {
  const query: Record<string, string> = { mode: state.mode };
  const q = nonBlankString(state.q);
  const player = nonBlankString(state.player);
  const filter = nonBlankString(state.filter);
  if (state.mode === 'name' && q) query.q = q;
  if (state.mode === 'player' && player) {
    query.player = player;
    if (filter) query.filter = filter;
  }
  return query;
};

/** Whether the current route query already holds exactly `expected`. */
export const isSameQuery = (current: LocationQuery, expected: Record<string, string>) =>
  Object.keys(current).length === Object.keys(expected).length &&
  Object.entries(expected).every(([k, v]) => current[k] === v);

const DECKLIST_STORAGE_KEY = 'tae_decklist_pending';

/**
 * Keeps the decklist for the search page, a blank one included: it clears the stored text, so
 * that `mode=decklist` never replays an older decklist.
 */
export const saveDecklist = (text: string) => {
  try {
    if (text.trim()) {
      sessionStorage.setItem(DECKLIST_STORAGE_KEY, text);
    } else {
      sessionStorage.removeItem(DECKLIST_STORAGE_KEY);
    }
  } catch {
    // sessionStorage full or unavailable: the decklist won't survive a reload.
  }
};

/** The decklist kept by `saveDecklist`, `null` if there is none. Left in place for a reload. */
export const loadDecklist = (): string | null => {
  try {
    return sessionStorage.getItem(DECKLIST_STORAGE_KEY);
  } catch {
    return null;
  }
};
