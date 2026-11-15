import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import {
  loadDecklist,
  parseSearchQuery,
  saveDecklist,
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

describe('searchListParams', () => {
  // The name search asks for the cards offered by any player (the Offre axis), a player's
  // collection for the cards that player offers.
  it.each<[string, SearchUrlState, SearchListParams | null]>([
    ['a submitted name', { mode: 'name', q: 'Sol Ring' }, { offered: true, q: 'Sol Ring' }],
    ['the name mode before any search', { mode: 'name' }, { offered: true }],
    ['a blank name', { mode: 'name', q: '  ' }, { offered: true }],
    ['a player', { mode: 'player', player: 'urza' }, { offered_by: 'urza' }],
    [
      'a player with a text filter',
      { mode: 'player', player: 'urza', filter: 'tutor' },
      { offered_by: 'urza', q: 'tutor' },
    ],
    [
      'a player, without the name searched before',
      { mode: 'player', player: 'urza', q: 'Sol Ring' },
      { offered_by: 'urza' },
    ],
    ['the player mode without player: no list', { mode: 'player', filter: 'tutor' }, null],
    ['the decklist mode: no list', { mode: 'decklist', q: 'Sol Ring', player: 'urza' }, null],
  ])('lists %s', (_label, state, expected) => {
    expect(searchListParams(state)).toEqual(expected);
  });
});

describe('searchSetsScope', () => {
  it.each<[string, SearchUrlState, SearchSetsParams | null]>([
    ['a submitted name', { mode: 'name', q: 'Sol Ring' }, { offered: true, q: 'Sol Ring' }],
    ['the name mode before any search', { mode: 'name' }, { offered: true }],
    ['a blank name', { mode: 'name', q: '  ' }, { offered: true }],
    ['a player', { mode: 'player', player: 'urza' }, { offered_by: 'urza' }],
    [
      'a player with a text filter, ignored',
      { mode: 'player', player: 'urza', filter: 'tutor' },
      { offered_by: 'urza' },
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
