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
 * Classes Tailwind de la couleur d'une rareté, convention MTG classique : `text` pour l'icône de set
 * du détail de carte et la lettre des règles d'échange, `border` pour les chips du filtre de rareté.
 * Chaînes littérales complètes, pour que Tailwind les détecte.
 */
export const RARITY_COLOR_CLASS: Record<RarityCode, { text: string; border: string }> = {
  C: { text: 'text-rarity-common', border: 'border-rarity-common' },
  U: { text: 'text-rarity-uncommon', border: 'border-rarity-uncommon' },
  R: { text: 'text-rarity-rare', border: 'border-rarity-rare' },
  M: { text: 'text-rarity-mythic', border: 'border-rarity-mythic' },
  S: { text: 'text-rarity-special', border: 'border-rarity-special' },
};
