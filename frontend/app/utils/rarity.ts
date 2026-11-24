import type { RarityCode } from '~/bindings/RarityCode';

export const RARITY_LABELS: Record<RarityCode, string> = {
  M: 'Mythique',
  R: 'Rare',
  U: 'Unco',
  C: 'Commune',
  S: 'Special',
};

/** Display order of the rarities, from most to least valuable. */
export const RARITY_ORDER: RarityCode[] = ['M', 'R', 'U', 'C', 'S'];

/**
 * Tailwind text-color class of a rarity, classic MTG convention: set icon of the card detail and
 * letter of the trade rules.
 */
export const RARITY_COLOR_CLASS: Record<RarityCode, string> = {
  C: 'text-rarity-common',
  U: 'text-rarity-uncommon',
  R: 'text-rarity-rare',
  M: 'text-rarity-mythic',
  S: 'text-rarity-special',
};
