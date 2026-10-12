import { describe, it, expect } from 'vitest';
import {
  COLLECTION_CRITERIA,
  PLAYER_SEARCH_CRITERIA,
  NAME_SEARCH_CRITERIA,
  centsToEuros,
  defaultCardCriteria,
  parseCardCriteria,
  toCardCriteriaQuery,
  type CardCriteria,
} from './card-criteria-url';

describe('criteria contexts', () => {
  const price = {
    value: 'trend',
    label: 'Prix',
    ascLabel: 'Prix croissant',
    descLabel: 'Prix décroissant',
  };
  const addedAt = {
    value: 'added_at',
    label: 'Date d’ajout',
    ascLabel: 'Plus ancien d’abord',
    descLabel: 'Plus récent d’abord',
  };

  it.each([
    ['the collection', COLLECTION_CRITERIA],
    ["a player's collection", PLAYER_SEARCH_CRITERIA],
  ])('offers the price and date added sorts in %s, date added first by default', (_l, context) => {
    expect(context.sorts).toEqual([price, addedAt]);
    expect(context.defaultSort).toEqual({ sort_by: 'added_at', sort_dir: 'desc' });
  });

  it('offers only the price sort in the search by name', () => {
    expect(NAME_SEARCH_CRITERIA.sorts).toEqual([price]);
    expect(NAME_SEARCH_CRITERIA.defaultSort).toEqual({ sort_by: 'trend', sort_dir: 'desc' });
  });

  it.each([
    ['the collection', COLLECTION_CRITERIA],
    ['the search by name', NAME_SEARCH_CRITERIA],
    ["a player's collection", PLAYER_SEARCH_CRITERIA],
  ])('carries the offset depth of the endpoint behind %s', (_label, context) => {
    expect(context.maxOffset).toBe(10_000);
  });
});

describe('parseCardCriteria', () => {
  it('falls back to the defaults on an empty query', () => {
    expect(parseCardCriteria({}, COLLECTION_CRITERIA)).toEqual({
      sort_by: 'added_at',
      sort_dir: 'desc',
      rarity: [],
      sets: [],
    });
  });

  it('reads every criterion', () => {
    expect(
      parseCardCriteria(
        {
          sort_by: 'trend',
          sort_dir: 'asc',
          rarity: 'M,R',
          sets: 'MH3,LTR',
          price_min: '150',
          price_max: '2000',
        },
        COLLECTION_CRITERIA,
      ),
    ).toEqual({
      sort_by: 'trend',
      sort_dir: 'asc',
      rarity: ['M', 'R'],
      sets: ['MH3', 'LTR'],
      price_min: 150,
      price_max: 2000,
    });
  });

  it.each([
    ['an unknown sort', { sort_by: 'oops' }, {}],
    ['a sort the context does not offer', { sort_by: 'added_at' }, {}],
    ['an unknown direction', { sort_dir: 'up' }, {}],
    ['a negative price', { price_min: '-100' }, {}],
    ['a decimal price', { price_max: '12.5' }, {}],
    ['a non-numeric price', { price_min: 'cheap' }, {}],
    ['a minimum above the maximum', { price_min: '500', price_max: '100' }, {}],
    ['a zero minimum (the default)', { price_min: '0' }, {}],
    ['repeated parameters', { sort_dir: ['asc', 'desc'], rarity: ['M', 'R'] }, {}],
    ['an unknown rarity among valid ones', { rarity: 'M,X,,R' }, { rarity: ['M', 'R'] }],
    ['duplicated rarities', { rarity: 'M,M' }, { rarity: ['M'] }],
    ['a malformed set among valid ones', { sets: 'MH3,<script>, ,LTR' }, { sets: ['MH3', 'LTR'] }],
    [
      'a valid price next to an invalid sort',
      { sort_by: 'x', price_max: '900' },
      { price_max: 900 },
    ],
  ])('ignores %s', (_label, query, expected) => {
    expect(parseCardCriteria(query, NAME_SEARCH_CRITERIA)).toEqual({
      ...defaultCardCriteria(NAME_SEARCH_CRITERIA),
      ...expected,
    });
  });

  it('uses the defaults of its context', () => {
    expect(parseCardCriteria({}, PLAYER_SEARCH_CRITERIA)).toMatchObject({
      sort_by: 'added_at',
      sort_dir: 'desc',
    });
    expect(parseCardCriteria({}, NAME_SEARCH_CRITERIA)).toMatchObject({
      sort_by: 'trend',
      sort_dir: 'desc',
    });
  });
});

describe('toCardCriteriaQuery', () => {
  it('omits the defaults', () => {
    expect(
      toCardCriteriaQuery(defaultCardCriteria(COLLECTION_CRITERIA), COLLECTION_CRITERIA),
    ).toEqual({});
  });

  it('writes each non-default criterion with the API names and units', () => {
    expect(
      toCardCriteriaQuery(
        {
          sort_by: 'trend',
          sort_dir: 'desc',
          rarity: ['M', 'R'],
          sets: ['MH3'],
          price_min: 150,
          price_max: 2000,
        },
        COLLECTION_CRITERIA,
      ),
    ).toEqual({
      sort_by: 'trend',
      rarity: 'M,R',
      sets: 'MH3',
      price_min: '150',
      price_max: '2000',
    });
  });

  it('writes only the direction when the sort field is the default', () => {
    expect(
      toCardCriteriaQuery(
        { ...defaultCardCriteria(COLLECTION_CRITERIA), sort_dir: 'asc' },
        COLLECTION_CRITERIA,
      ),
    ).toEqual({ sort_dir: 'asc' });
  });

  it.each<CardCriteria>([
    { sort_by: 'trend', sort_dir: 'asc', rarity: ['U'], sets: ['MH3', 'LTR'], price_max: 900 },
    { sort_by: 'added_at', sort_dir: 'desc', rarity: [], sets: [], price_min: 1 },
    { sort_by: 'trend', sort_dir: 'desc', rarity: ['M', 'R', 'U', 'C', 'S'], sets: [] },
  ])('round-trips %j through the URL', (criteria) => {
    const query = toCardCriteriaQuery(criteria, COLLECTION_CRITERIA);
    expect(parseCardCriteria(query, COLLECTION_CRITERIA)).toEqual(criteria);
  });
});

describe('centsToEuros', () => {
  it.each([
    [undefined, undefined],
    [0, 0],
    [1550, 15.5],
  ])('converts %j cents', (cents, euros) => {
    expect(centsToEuros(cents)).toBe(euros);
  });
});
