import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { CardCriteria } from './card-criteria-url';
import {
  isSameQuery,
  loadDecklist,
  parseSearchCriteria,
  parseSearchQuery,
  saveDecklist,
  toSearchPageQuery,
  toSearchQuery,
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

describe('search criteria', () => {
  it('reads the criteria of a name search, with only the price sort', () => {
    expect(
      parseSearchCriteria({ q: 'Sol Ring', sort_by: 'added_at', sort_dir: 'asc', rarity: 'M' }),
    ).toEqual({ sort_by: 'trend', sort_dir: 'asc', rarity: ['M'], sets: [] });
  });

  it("reads the criteria of a player's collection, sorted by date added by default", () => {
    expect(parseSearchCriteria({ player: 'urza', price_max: '500' })).toEqual({
      sort_by: 'added_at',
      sort_dir: 'desc',
      rarity: [],
      sets: [],
      price_max: 500,
    });
  });

  it.each([
    ['the decklist mode', { mode: 'decklist', rarity: 'M' }],
    ['the player mode without a player', { mode: 'player', rarity: 'M' }],
  ])('ignores the criteria in %s, which shows none', (_label, query) => {
    expect(parseSearchCriteria(query)).toEqual({
      sort_by: 'trend',
      sort_dir: 'desc',
      rarity: [],
      sets: [],
    });
  });

  it('writes the criteria next to the search, defaults omitted', () => {
    expect(
      toSearchPageQuery(
        { mode: 'player', player: 'urza' },
        { sort_by: 'trend', sort_dir: 'desc', rarity: ['R'], sets: [], price_min: 100 },
      ),
    ).toEqual({ mode: 'player', player: 'urza', sort_by: 'trend', rarity: 'R', price_min: '100' });
    expect(
      toSearchPageQuery(
        { mode: 'name' },
        { sort_by: 'trend', sort_dir: 'desc', rarity: [], sets: [] },
      ),
    ).toEqual({ mode: 'name' });
  });

  it('leaves the criteria out of a search that shows none', () => {
    expect(
      toSearchPageQuery(
        { mode: 'decklist' },
        { sort_by: 'trend', sort_dir: 'asc', rarity: ['R'], sets: ['MH3'] },
      ),
    ).toEqual({ mode: 'decklist' });
  });

  it('round-trips a search and its criteria through the URL', () => {
    const state: SearchUrlState = { mode: 'name', q: 'Sol Ring' };
    const criteria: CardCriteria = {
      sort_by: 'trend',
      sort_dir: 'asc',
      rarity: ['M', 'R'],
      sets: ['MH3'],
      price_min: 100,
      price_max: 2000,
    };
    const query = toSearchPageQuery(state, criteria);
    expect(parseSearchQuery(query)).toEqual(state);
    expect(parseSearchCriteria(query)).toEqual(criteria);
  });
});

describe('isSameQuery', () => {
  it('matches the same parameters', () => {
    expect(isSameQuery({ q: 'a', mode: 'name' }, { mode: 'name', q: 'a' })).toBe(true);
  });

  it.each([
    ['a missing parameter', { mode: 'name' }],
    ['an extra parameter', { mode: 'name', q: 'a', player: 'urza' }],
    ['a different value', { mode: 'name', q: 'b' }],
  ])('rejects %s', (_label, current) => {
    expect(isSameQuery(current, { mode: 'name', q: 'a' })).toBe(false);
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
