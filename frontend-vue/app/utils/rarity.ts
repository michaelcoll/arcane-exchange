import type { RarityCode } from '~/bindings/RarityCode';

export const RARITY_LABELS: Record<RarityCode, string> = {
  M: 'Mythique',
  R: 'Rare',
  U: 'Unco',
  C: 'Commune',
  S: 'Special',
};

/** Ordre d'affichage des raretés, du plus au moins précieux. */
export const RARITY_ORDER: RarityCode[] = ['M', 'R', 'U', 'C', 'S'];
