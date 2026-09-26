import type { CardSize } from '../composables/useCardPageSize';

/**
 * A display preference kept in localStorage across reloads: the key, the values the page still
 * offers, and the default. Unlike the criteria (in the URL), it follows the browser, not the link.
 */
export interface Preference<T extends string> {
  key: string;
  values: readonly T[];
  fallback: T;
}

const definePreference = <T extends string>(
  key: string,
  values: readonly T[],
  fallback: T,
): Preference<T> => ({ key, values, fallback });

/** Shared by the search and the collection: the size picked on one page applies to the other. */
export const CARD_SIZE_PREF = definePreference<CardSize>('tae_card_size', ['sm', 'md', 'lg'], 'md');

export const COLLECTION_VIEW_PREF = definePreference(
  'tae_collection_view',
  ['grid', 'list'] as const,
  'grid',
);

export const COLLECTION_GRAPH_PREF = definePreference(
  'tae_collection_graph',
  ['compact', 'expanded'] as const,
  'compact',
);

export const GRAPH_RANGES = ['30 j', '3 m', '1 an', 'Max'] as const;
export type GraphRange = (typeof GRAPH_RANGES)[number];

export const COLLECTION_GRAPH_RANGE_PREF = definePreference<GraphRange>(
  'tae_collection_graph_range',
  GRAPH_RANGES,
  '30 j',
);

/** The stored value, or the default when there is none, it is no longer offered, or the storage fails. */
export const loadPreference = <T extends string>(pref: Preference<T>): T => {
  try {
    const stored = localStorage.getItem(pref.key);
    return pref.values.includes(stored as T) ? (stored as T) : pref.fallback;
  } catch {
    return pref.fallback;
  }
};

export const savePreference = <T extends string>(pref: Preference<T>, value: T) => {
  try {
    localStorage.setItem(pref.key, value);
  } catch {
    // localStorage full or unavailable: the preference won't survive a reload.
  }
};
