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

/**
 * Classe Tailwind (couleur de texte) d'une rareté, convention MTG classique : icône de set du détail
 * de carte et lettre des règles d'échange.
 */
export const RARITY_COLOR_CLASS: Record<RarityCode, string> = {
  C: 'text-rarity-common',
  U: 'text-rarity-uncommon',
  R: 'text-rarity-rare',
  M: 'text-rarity-mythic',
  S: 'text-rarity-special',
};
