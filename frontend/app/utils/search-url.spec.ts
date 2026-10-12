import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import { NAME_SEARCH_CRITERIA, PLAYER_SEARCH_CRITERIA } from './card-criteria-url';
import {
  loadDecklist,
  parseSearchQuery,
  saveDecklist,
  searchCriteriaContext,
  searchListParams,
  searchRoute,
  searchSetsScope,
  toSearchQuery,
  type SearchListParams,
  type SearchUrlState,
} from './search-url';

const storage = new Map<string, string>();
vi.stubGlobal('sessionStorage', {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => storage.set(key, value),
  removeItem: (key: string) => storage.delete(key),
});

describe('parseSearchQuery', () => {
  it.each([
    ['an empty query', {}, { mode: 'name' }],
    ['a name search', { mode: 'name', q: 'Sol Ring' }, { mode: 'name', q: 'Sol Ring' }],
    ['a name without mode (home page)', { q: 'Sol Ring' }, { mode: 'name', q: 'Sol Ring' }],
    ['a blank name', { mode: 'name', q: '  ' }, { mode: 'name' }],
    ['a player without mode (home, trade)', { player: 'urza' }, { mode: 'player', player: 'urza' }],
    [
      'a player with a filter',
      { mode: 'player', player: 'urza', filter: 'tutor' },
      { mode: 'player', player: 'urza', filter: 'tutor' },
    ],
    ['the player mode alone', { mode: 'player' }, { mode: 'player' }],
    ['a filter without player', { mode: 'player', filter: 'tutor' }, { mode: 'player' }],
    ['the decklist mode', { mode: 'decklist', q: 'ignored' }, { mode: 'decklist' }],
    ['a player in another mode', { mode: 'name', player: 'urza' }, { mode: 'name' }],
    ['an unknown mode', { mode: 'oops', q: 'Sol Ring' }, { mode: 'name', q: 'Sol Ring' }],
    ['repeated parameters', { q: ['a', 'b'] }, { mode: 'name' }],
  ])('reads %s', (_label, query, expected) => {
    expect(parseSearchQuery(query)).toEqual(expected);
  });
});

describe('toSearchQuery', () => {
  it.each<[string, SearchUrlState, Record<string, string>]>([
    ['a name search', { mode: 'name', q: 'Sol Ring' }, { mode: 'name', q: 'Sol Ring' }],
    ['a blank name', { mode: 'name', q: ' ' }, { mode: 'name' }],
    [
      'a player with a filter',
      { mode: 'player', player: 'urza', filter: 'tutor' },
      { mode: 'player', player: 'urza', filter: 'tutor' },
    ],
    ['a filter without player', { mode: 'player', filter: 'tutor' }, { mode: 'player' }],
    ['the decklist mode', { mode: 'decklist', q: 'x', player: 'y' }, { mode: 'decklist' }],
  ])('writes %s', (_label, state, expected) => {
    expect(toSearchQuery(state)).toEqual(expected);
  });

  it.each<SearchUrlState>([
    { mode: 'name', q: 'Sol Ring' },
    { mode: 'player', player: 'urza', filter: 'tutor' },
    { mode: 'decklist' },
  ])('round-trips %j through the URL', (state) => {
    expect(parseSearchQuery(toSearchQuery(state))).toEqual(state);
  });
});

describe('searchRoute', () => {
  it.each<[string, SearchUrlState, Record<string, string>]>([
    [
      "a player's collection",
      { mode: 'player', player: 'urza' },
      { mode: 'player', player: 'urza' },
    ],
    ['a search by name', { mode: 'name', q: 'Sol Ring' }, { mode: 'name', q: 'Sol Ring' }],
    ['the name mode without a name', { mode: 'name', q: '' }, { mode: 'name' }],
    ['the decklist mode', { mode: 'decklist' }, { mode: 'decklist' }],
  ])('links to %s on the search page', (_label, state, query) => {
    expect(searchRoute(state)).toEqual({ path: '/search', query });
  });

  it.each<SearchUrlState>([
    { mode: 'player', player: 'urza' },
    { mode: 'name', q: 'Sol Ring' },
    { mode: 'decklist' },
  ])('opens the search %j it was built from', (state) => {
    expect(parseSearchQuery(searchRoute(state).query)).toEqual(state);
  });
});

describe('searchCriteriaContext', () => {
  it.each<[string, SearchUrlState, typeof NAME_SEARCH_CRITERIA]>([
    ['a search by name', { mode: 'name', q: 'Sol Ring' }, NAME_SEARCH_CRITERIA],
    ["a player's collection", { mode: 'player', player: 'urza' }, PLAYER_SEARCH_CRITERIA],
    ['the player mode without player', { mode: 'player' }, NAME_SEARCH_CRITERIA],
    ['the decklist mode', { mode: 'decklist' }, NAME_SEARCH_CRITERIA],
  ])('gives %s its sorts', (_label, state, expected) => {
    expect(searchCriteriaContext(state)).toBe(expected);
  });
});

describe('searchListParams', () => {
  it.each<[string, SearchUrlState, SearchListParams | null]>([
    ['a submitted name', { mode: 'name', q: 'Sol Ring' }, { q: 'Sol Ring' }],
    ['the name mode before any search', { mode: 'name' }, {}],
    ['a blank name', { mode: 'name', q: '  ' }, {}],
    ['a player', { mode: 'player', player: 'urza' }, { player_username: 'urza' }],
    [
      'a player with a text filter',
      { mode: 'player', player: 'urza', filter: 'tutor' },
      { player_username: 'urza', q: 'tutor' },
    ],
    [
      'a player, without the name searched before',
      { mode: 'player', player: 'urza', q: 'Sol Ring' },
      { player_username: 'urza' },
    ],
    ['the player mode without player: no list', { mode: 'player', filter: 'tutor' }, null],
    ['the decklist mode: no list', { mode: 'decklist', q: 'Sol Ring', player: 'urza' }, null],
  ])('lists %s', (_label, state, expected) => {
    expect(searchListParams(state)).toEqual(expected);
  });
});

describe('searchSetsScope', () => {
  it.each<[string, SearchUrlState, SearchSetsParams | null]>([
    ['a submitted name', { mode: 'name', q: 'Sol Ring' }, { q: 'Sol Ring' }],
    ['the name mode before any search', { mode: 'name' }, {}],
    ['a blank name', { mode: 'name', q: '  ' }, {}],
    ['a player', { mode: 'player', player: 'urza' }, { player_username: 'urza' }],
    [
      'a player with a text filter, ignored',
      { mode: 'player', player: 'urza', filter: 'tutor' },
      { player_username: 'urza' },
    ],
    ['the player mode without player', { mode: 'player' }, null],
    ['the decklist mode', { mode: 'decklist' }, null],
  ])('scopes %s', (_label, state, expected) => {
    expect(searchSetsScope(state)).toEqual(expected);
  });
});

describe('decklist storage', () => {
  beforeEach(() => {
    storage.clear();
  });

  it('keeps the decklist across reads', () => {
    saveDecklist('1x Sol Ring');
    expect(loadDecklist()).toBe('1x Sol Ring');
    expect(loadDecklist()).toBe('1x Sol Ring');
  });

  it('clears a stored decklist when a blank one is saved', () => {
    saveDecklist('1x Sol Ring');
    saveDecklist('  ');
    expect(loadDecklist()).toBeNull();
  });
});
