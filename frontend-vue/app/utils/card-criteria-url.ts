import type { LocationQuery } from 'vue-router';
import type { RarityCode } from '~/bindings/RarityCode';
import type { SortBy } from '~/bindings/SortBy';
import type { SortDir } from '~/bindings/SortDir';
import { RARITY_ORDER } from './rarity';

/**
 * The criteria of a card list (search, collection) as the URL carries them, so that a reload
 * replays them. Names and units follow the API parameters: prices in cents, lists joined by
 * commas. The criteria left at their default stay out of the URL.
 */
export interface CardCriteria {
  sort_by: SortBy;
  sort_dir: SortDir;
  rarity: RarityCode[];
  sets: string[];
  price_min?: number;
  price_max?: number;
}

/** What a page offers: the sorts it lists, and the one it starts with. */
export interface CriteriaContext {
  sorts: readonly SortBy[];
  defaultSort: { sort_by: SortBy; sort_dir: SortDir };
}

export const COLLECTION_CRITERIA: CriteriaContext = {
  sorts: ['trend', 'added_at'],
  defaultSort: { sort_by: 'added_at', sort_dir: 'desc' },
};

/** The search by name, across every player: only the price sort. */
export const NAME_SEARCH_CRITERIA: CriteriaContext = {
  sorts: ['trend'],
  defaultSort: { sort_by: 'trend', sort_dir: 'desc' },
};

/** The collection of one player: `added_at` only makes sense scoped to a single player. */
export const PLAYER_SEARCH_CRITERIA: CriteriaContext = {
  sorts: ['trend', 'added_at'],
  defaultSort: { sort_by: 'added_at', sort_dir: 'desc' },
};

const SORT_DIRS: readonly SortDir[] = ['asc', 'desc'];
const SET_CODE = /^[A-Za-z0-9]+$/;
const CENTS = /^\d+$/;

export const defaultCardCriteria = (context: CriteriaContext): CardCriteria => ({
  ...context.defaultSort,
  rarity: [],
  sets: [],
});

/** A single value of the query; a repeated parameter counts as invalid. */
const single = (v: LocationQuery[string] | undefined): string | undefined =>
  typeof v === 'string' ? v : undefined;

const list = (v: LocationQuery[string] | undefined): string[] => [
  ...new Set(
    (single(v) ?? '')
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean),
  ),
];

const cents = (v: LocationQuery[string] | undefined): number | undefined => {
  const s = single(v);
  return s && CENTS.test(s) ? Number(s) : undefined;
};

/**
 * Reads the criteria from the URL. Each invalid value (unknown sort, missing rarity, negative
 * price…) falls back to its default on its own; the valid ones still apply.
 */
export const parseCardCriteria = (query: LocationQuery, context: CriteriaContext): CardCriteria => {
  const criteria = defaultCardCriteria(context);

  const sortBy = single(query.sort_by);
  if (context.sorts.includes(sortBy as SortBy)) criteria.sort_by = sortBy as SortBy;
  const sortDir = single(query.sort_dir);
  if (SORT_DIRS.includes(sortDir as SortDir)) criteria.sort_dir = sortDir as SortDir;

  criteria.rarity = list(query.rarity).filter((r): r is RarityCode =>
    RARITY_ORDER.includes(r as RarityCode),
  );
  criteria.sets = list(query.sets).filter((s) => SET_CODE.test(s));

  // A minimum of 0 is no bound at all; a range upside down has no bound to keep.
  const min = cents(query.price_min) || undefined;
  const max = cents(query.price_max);
  if (min == null || max == null || min <= max) {
    if (min != null) criteria.price_min = min;
    if (max != null) criteria.price_max = max;
  }
  return criteria;
};

/** Builds the URL query of the criteria, the inverse of `parseCardCriteria`. */
export const toCardCriteriaQuery = (
  criteria: CardCriteria,
  context: CriteriaContext,
): Record<string, string> => {
  const query: Record<string, string> = {};
  if (criteria.sort_by !== context.defaultSort.sort_by) query.sort_by = criteria.sort_by;
  if (criteria.sort_dir !== context.defaultSort.sort_dir) query.sort_dir = criteria.sort_dir;
  if (criteria.rarity.length) query.rarity = criteria.rarity.join(',');
  if (criteria.sets.length) query.sets = criteria.sets.join(',');
  if (criteria.price_min) query.price_min = String(criteria.price_min);
  if (criteria.price_max != null) query.price_max = String(criteria.price_max);
  return query;
};

/** A price bound of the criteria (cents) in the unit of the price slider (euros). */
export const centsToEuros = (cents?: number): number | undefined =>
  cents == null ? undefined : cents / 100;

/**
 * The collection as its URL carries it: the text filter and the criteria. The collection is
 * private, its URL only serves its owner (reload, bookmark).
 */
export interface CollectionUrlState extends CardCriteria {
  q: string;
}

export const parseCollectionQuery = (query: LocationQuery): CollectionUrlState => {
  const q = single(query.q) ?? '';
  return { q: q.trim() ? q : '', ...parseCardCriteria(query, COLLECTION_CRITERIA) };
};

export const toCollectionQuery = ({
  q,
  ...criteria
}: CollectionUrlState): Record<string, string> => ({
  ...(q.trim() ? { q } : {}),
  ...toCardCriteriaQuery(criteria, COLLECTION_CRITERIA),
});
