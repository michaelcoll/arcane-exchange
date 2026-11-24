import type { LocationQuery } from 'vue-router';
import type { SearchParams } from '~/bindings/SearchParams';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';

export const SEARCH_MODES = ['name', 'player'] as const;
export type SearchMode = (typeof SEARCH_MODES)[number];

export const isSearchMode = (v: unknown): v is SearchMode => SEARCH_MODES.includes(v as SearchMode);

/**
 * The search as the URL of `/search` carries it, so that a reload replays it. Its criteria (sort,
 * rarity, set, price) sit next to it (see `useCardList`); the card size is a display preference,
 * kept in localStorage.
 *
 * - `q`: the submitted card name (`name` mode);
 * - `player`: the browsed player (`player` mode), and `filter` the text filter on their collection;
 * - `owned`: the Possession axis, `true` for the impressions I own, `false` for those I do not,
 *   absent for all of them;
 * - `offered`: the Offre axis of the name search, `true` for the impressions some player offers,
 *   absent (the default) for the whole Catalogue. A browsed player is the Offre axis of the
 *   player mode.
 */
export interface SearchUrlState {
  mode: SearchMode;
  q?: string;
  player?: string;
  filter?: string;
  owned?: boolean;
  offered?: boolean;
}

const nonBlankString = (v: unknown): string | undefined =>
  typeof v === 'string' && v.trim() ? v : undefined;

const booleanOf = (v: unknown): boolean | undefined =>
  v === 'true' ? true : v === 'false' ? false : undefined;

/**
 * Reads the search from the URL. `player` without a `mode` (the links from the home page and a
 * trade) opens that player's collection; `q` only counts in `name` mode, `filter` only with a
 * player.
 */
export const parseSearchQuery = (query: LocationQuery): SearchUrlState => {
  const mode = isSearchMode(query.mode) ? query.mode : undefined;
  const player = nonBlankString(query.player);
  const owned = booleanOf(query.owned);

  if (player && (mode === 'player' || !mode)) {
    const filter = nonBlankString(query.filter);
    return {
      mode: 'player',
      player,
      ...(filter && { filter }),
      ...(owned !== undefined && { owned }),
    };
  }
  if (!mode || mode === 'name') {
    const q = nonBlankString(query.q);
    return {
      mode: 'name',
      ...(q && { q }),
      ...(owned !== undefined && { owned }),
      ...(booleanOf(query.offered) === true && { offered: true }),
    };
  }
  return { mode: 'player' };
};

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
  if (state.mode === 'name' || (state.mode === 'player' && player)) {
    if (state.owned !== undefined) query.owned = String(state.owned);
  }
  if (state.mode === 'name' && state.offered === true) query.offered = 'true';
  return query;
};

export const searchRoute = (state: SearchUrlState) => ({
  path: '/search',
  query: toSearchQuery(state),
});

export type SearchListParams = Pick<SearchParams, 'q' | 'owned' | 'offered' | 'offered_by'>;

/**
 * The request of the card list of a search (`GET /search/card`): the scope of its set facet.
 * `null` where the search shows no list (player mode before a player is chosen).
 */
export const searchListParams = (state: SearchUrlState): SearchListParams | null =>
  searchSetsScope(state);

/**
 * The scope of the set facet (`GET /search/card/sets`) for a search: the Possession axis, the
 * Offre axis (the whole Catalogue for a name, unless the cards offered by any player are asked;
 * those of the browsed player in player mode) and the text (the submitted name, or the text
 * filter on a browsed player's collection). `null` where the search shows no list, so no facet.
 */
export const searchSetsScope = (state: SearchUrlState): SearchSetsParams | null => {
  const owned = state.owned !== undefined ? { owned: state.owned } : {};
  if (state.mode === 'name') {
    const q = nonBlankString(state.q);
    return { ...owned, ...(state.offered === true && { offered: true }), ...(q && { q }) };
  }
  if (state.mode === 'player' && state.player) {
    const q = nonBlankString(state.filter);
    return { ...owned, offered_by: state.player, ...(q && { q }) };
  }
  return null;
};
