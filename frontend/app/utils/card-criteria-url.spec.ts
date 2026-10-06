import { describe, it, expect } from 'vitest';
import {
  COLLECTION_CRITERIA,
  PLAYER_SEARCH_CRITERIA,
  NAME_SEARCH_CRITERIA,
  centsToEuros,
  defaultCardCriteria,
  parseCardCriteria,
  parseCollectionQuery,
  toCardCriteriaQuery,
  toCollectionQuery,
  type CardCriteria,
} from './card-criteria-url';

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

describe('collection query', () => {
  it('reads the text filter with the criteria', () => {
    expect(parseCollectionQuery({ q: 'Sol Ring', rarity: 'M' })).toEqual({
      ...defaultCardCriteria(COLLECTION_CRITERIA),
      q: 'Sol Ring',
      rarity: ['M'],
    });
  });

  it('drops a blank text filter', () => {
    expect(parseCollectionQuery({ q: '  ' })).toEqual({
      ...defaultCardCriteria(COLLECTION_CRITERIA),
      q: '',
    });
    expect(toCollectionQuery({ ...defaultCardCriteria(COLLECTION_CRITERIA), q: ' ' })).toEqual({});
  });

  it('round-trips through the URL', () => {
    const state = {
      q: 'tutor',
      sort_by: 'trend' as const,
      sort_dir: 'asc' as const,
      rarity: ['M' as const],
      sets: ['MH3'],
      price_min: 100,
    };
    expect(parseCollectionQuery(toCollectionQuery(state))).toEqual(state);
  });
});
