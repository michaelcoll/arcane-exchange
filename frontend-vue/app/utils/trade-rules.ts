import type { RarityCode } from '~/bindings/RarityCode';

/* Règles de mise à l'échange (écran Profil).
 * Les données ci-dessous sont encore mockées : distribution de la collection et
 * binders ManaBox viendront du backend (import ManaBox + stats collection). */

/** Réglage d'une rareté : ouverte à l'échange, et nombre d'exemplaires toujours gardés. */
export interface RarityRule {
  on: boolean;
  keep: number;
}

export type TradeRuleRarity = Extract<RarityCode, 'M' | 'R' | 'U' | 'C'>;

export interface RarityDistribution {
  code: TradeRuleRarity;
  label: string;
  /** nombre de cartes uniques possédées en N exemplaires : { 1: 62, 2: 9, … } */
  dist: Record<number, number>;
}

export interface Binder {
  key: string;
  name: string;
  cards: number;
  /** part des exemplaires de chaque rareté rangée dans ce binder (somme = 1 par rareté) */
  weights: Record<TradeRuleRarity, number>;
}

/** Répartition démo : 1 248 cartes uniques, 1 896 exemplaires. */
export const RARITY_DISTRIBUTION: RarityDistribution[] = [
  { code: 'M', label: 'Mythique', dist: { 1: 62, 2: 9, 3: 3 } },
  { code: 'R', label: 'Rare', dist: { 1: 198, 2: 48, 3: 12, 4: 3 } },
  { code: 'U', label: 'Unco', dist: { 1: 240, 2: 96, 3: 35, 4: 12 } },
  { code: 'C', label: 'Commune', dist: { 1: 300, 2: 140, 3: 60, 4: 30 } },
];

export const BINDERS: Binder[] = [
  {
    key: 'trade',
    name: 'Trade Binder',
    cards: 412,
    weights: { M: 0.15, R: 0.35, U: 0.45, C: 0.3 },
  },
  { key: 'bulk', name: 'Bulk', cards: 498, weights: { M: 0.02, R: 0.1, U: 0.3, C: 0.55 } },
  { key: 'decks', name: 'Decks', cards: 286, weights: { M: 0.55, R: 0.3, U: 0.12, C: 0.08 } },
  { key: 'edh', name: 'EDH staples', cards: 118, weights: { M: 0.25, R: 0.2, U: 0.08, C: 0.02 } },
  { key: 'none', name: 'Non classé', cards: 82, weights: { M: 0.03, R: 0.05, U: 0.05, C: 0.05 } },
];

export const DEFAULT_RARITY_RULES: Record<TradeRuleRarity, RarityRule> = {
  M: { on: false, keep: 1 },
  R: { on: true, keep: 1 },
  U: { on: true, keep: 0 },
  C: { on: true, keep: 0 },
};

export const MAX_KEPT_COPIES = 4;

/** Nombre de cartes uniques d'une rareté. */
export const uniqueOf = (r: RarityDistribution): number =>
  Object.values(r.dist).reduce((a, b) => a + b, 0);

/** Nombre d'exemplaires (toutes copies confondues) d'une rareté. */
export const copiesOf = (r: RarityDistribution): number =>
  Object.entries(r.dist).reduce((s, [q, n]) => s + Number(q) * n, 0);

/** Exemplaires proposables : ce qui dépasse les « copies gardées », pondéré par la part des binders retenus. */
export const eligibleCopies = (r: RarityDistribution, keep: number, share = 1): number => {
  const raw = Object.entries(r.dist).reduce(
    (s, [q, n]) => s + n * Math.max(0, Number(q) - keep),
    0,
  );
  return Math.round(raw * share);
};

/** Part des exemplaires d'une rareté couverte par les binders sélectionnés. */
export const binderFactor = (rarity: TradeRuleRarity, selected: string[]): number =>
  BINDERS.reduce((s, b) => s + (selected.includes(b.key) ? b.weights[rarity] : 0), 0);

export const TOTAL_COPIES = RARITY_DISTRIBUTION.reduce((s, r) => s + copiesOf(r), 0);

export const fmtInt = (n: number): string => Math.round(n).toLocaleString('fr-FR');
