import { describe, it, expect } from 'vitest';
import type { SearchSetsParams } from '~/bindings/SearchSetsParams';
import {
  parseSearchQuery,
  searchListParams,
  searchRoute,
  searchSetsScope,
  toSearchQuery,
  type SearchListParams,
  type SearchUrlState,
} from './search-url';

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
    [
      'the retired decklist mode, as the name search',
      { mode: 'decklist', q: 'Sol Ring' },
      { mode: 'name', q: 'Sol Ring' },
    ],
    ['a player in another mode', { mode: 'name', player: 'urza' }, { mode: 'name' }],
    ['an unknown mode', { mode: 'oops', q: 'Sol Ring' }, { mode: 'name', q: 'Sol Ring' }],
    ['repeated parameters', { q: ['a', 'b'] }, { mode: 'name' }],
    [
      'the owned impressions',
      { q: 'Sol Ring', owned: 'true' },
      { mode: 'name', q: 'Sol Ring', owned: true },
    ],
    ['the impressions not owned', { owned: 'false' }, { mode: 'name', owned: false }],
    ['an unknown possession', { owned: 'maybe' }, { mode: 'name' }],
    ['every impression of the Catalogue', { offered: 'false' }, { mode: 'name', offered: false }],
    ['the default offer written out', { offered: 'true' }, { mode: 'name' }],
    [
      'the possession of a browsed player',
      { player: 'urza', owned: 'false' },
      { mode: 'player', player: 'urza', owned: false },
    ],
    [
      'the offer of a browsed player, ignored',
      { player: 'urza', offered: 'false' },
      { mode: 'player', player: 'urza' },
    ],
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
    ['the owned impressions', { mode: 'name', owned: true }, { mode: 'name', owned: 'true' }],
    ['the impressions not owned', { mode: 'name', owned: false }, { mode: 'name', owned: 'false' }],
    ['the whole Catalogue', { mode: 'name', offered: false }, { mode: 'name', offered: 'false' }],
    ['the default offer', { mode: 'name', offered: true }, { mode: 'name' }],
    [
      'the possession of a browsed player',
      { mode: 'player', player: 'urza', owned: true },
      { mode: 'player', player: 'urza', owned: 'true' },
    ],
    [
      'the offer of a browsed player, left out',
      { mode: 'player', player: 'urza', offered: false },
      { mode: 'player', player: 'urza' },
    ],
  ])('writes %s', (_label, state, expected) => {
    expect(toSearchQuery(state)).toEqual(expected);
  });

  it.each<SearchUrlState>([
    { mode: 'name', q: 'Sol Ring', owned: false, offered: false },
    { mode: 'player', player: 'urza', owned: true },
    { mode: 'name', q: 'Sol Ring' },
    { mode: 'player', player: 'urza', filter: 'tutor' },
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
  ])('links to %s on the search page', (_label, state, query) => {
    expect(searchRoute(state)).toEqual({ path: '/search', query });
  });

  it.each<SearchUrlState>([
    { mode: 'player', player: 'urza' },
    { mode: 'name', q: 'Sol Ring' },
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
    [
      'the impressions I miss that somebody offers',
      { mode: 'name', owned: false },
      { offered: true, owned: false },
    ],
    [
      'the impressions I own, offered or not',
      { mode: 'name', q: 'Sol Ring', owned: true, offered: false },
      { owned: true, q: 'Sol Ring' },
    ],
    ['the whole Catalogue', { mode: 'name', offered: false }, {}],
    [
      'the impressions of a player I miss',
      { mode: 'player', player: 'urza', owned: false },
      { offered_by: 'urza', owned: false },
    ],
    ['the player mode without player: no list', { mode: 'player', filter: 'tutor' }, null],
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
      'a player with a text filter',
      { mode: 'player', player: 'urza', filter: 'tutor' },
      { offered_by: 'urza', q: 'tutor' },
    ],
    ['the whole Catalogue', { mode: 'name', q: 'Sol Ring', offered: false }, { q: 'Sol Ring' }],
    ['a possession', { mode: 'name', owned: false }, { offered: true, owned: false }],
    [
      'a possession on a player',
      { mode: 'player', player: 'urza', owned: true },
      { offered_by: 'urza', owned: true },
    ],
    ['the player mode without player', { mode: 'player' }, null],
  ])('scopes %s', (_label, state, expected) => {
    expect(searchSetsScope(state)).toEqual(expected);
  });
});
