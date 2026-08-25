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

/** Classe Tailwind (couleur de texte) de l'icône de set selon la rareté de la carte, convention MTG classique. */
export const RARITY_ICON_COLOR_CLASS: Record<RarityCode, string> = {
  C: 'text-[var(--rarity-common)]',
  U: 'text-[var(--rarity-uncommon)]',
  R: 'text-[var(--rarity-rare)]',
  M: 'text-[var(--rarity-mythic)]',
  S: 'text-[var(--rarity-special)]',
};
