import type { LocationQuery } from 'vue-router';
import type { RarityCode } from '~/bindings/RarityCode';
import type { SortBy } from '~/bindings/SortBy';
import type { SortDir } from '~/bindings/SortDir';
import { COLLECTION_MAX_OFFSET, SEARCH_MAX_OFFSET } from './pagination';
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

/** A sort a card list offers: its value, and its labels (idle, then active in each direction). */
export interface SortOption {
  value: SortBy;
  label: string;
  ascLabel: string;
  descLabel: string;
}

/**
 * What a card list offers, declared once per context: the sorts it lists (values and labels),
 * the one it starts with, and how deep its endpoint paginates (see `canLoadPage`).
 */
export interface CriteriaContext {
  sorts: readonly SortOption[];
  defaultSort: { sort_by: SortBy; sort_dir: SortDir };
  maxOffset: number;
}

const PRICE_SORT: SortOption = {
  value: 'trend',
  label: 'Prix',
  ascLabel: 'Prix croissant',
  descLabel: 'Prix décroissant',
};

const ADDED_AT_SORT: SortOption = {
  value: 'added_at',
  label: 'Date d’ajout',
  ascLabel: 'Plus ancien d’abord',
  descLabel: 'Plus récent d’abord',
};

export const COLLECTION_CRITERIA: CriteriaContext = {
  sorts: [PRICE_SORT, ADDED_AT_SORT],
  defaultSort: { sort_by: 'added_at', sort_dir: 'desc' },
  maxOffset: COLLECTION_MAX_OFFSET,
};

/** The search by name, across every player: only the price sort. */
export const NAME_SEARCH_CRITERIA: CriteriaContext = {
  sorts: [PRICE_SORT],
  defaultSort: { sort_by: 'trend', sort_dir: 'desc' },
  maxOffset: SEARCH_MAX_OFFSET,
};

/** The collection of one player: `added_at` only makes sense scoped to a single player. */
export const PLAYER_SEARCH_CRITERIA: CriteriaContext = {
  sorts: [PRICE_SORT, ADDED_AT_SORT],
  defaultSort: { sort_by: 'added_at', sort_dir: 'desc' },
  maxOffset: SEARCH_MAX_OFFSET,
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
  const sort = context.sorts.find((s) => s.value === sortBy);
  if (sort) criteria.sort_by = sort.value;
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
