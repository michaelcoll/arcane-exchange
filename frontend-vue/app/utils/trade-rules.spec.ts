import { describe, it, expect } from 'vitest';
import {
  BINDER_WEIGHTS,
  FALLBACK_BINDER_WEIGHT,
  RARITY_DISTRIBUTION,
  TOTAL_COPIES,
  binderFactor,
  copiesOf,
  eligibleCopies,
  uniqueOf,
  type RarityDistribution,
} from './trade-rules';

const rarity: RarityDistribution = { code: 'R', label: 'Rare', dist: { 1: 10, 2: 5, 3: 2 } };

describe('uniqueOf / copiesOf', () => {
  it('compte les cartes uniques', () => {
    expect(uniqueOf(rarity)).toBe(17);
  });

  it('compte les exemplaires', () => {
    expect(copiesOf(rarity)).toBe(10 * 1 + 5 * 2 + 2 * 3);
  });

  it('somme les exemplaires de la collection démo', () => {
    expect(TOTAL_COPIES).toBe(RARITY_DISTRIBUTION.reduce((s, r) => s + copiesOf(r), 0));
  });
});

describe('eligibleCopies', () => {
  it('propose tout quand aucune copie n’est gardée', () => {
    expect(eligibleCopies(rarity, 0)).toBe(copiesOf(rarity));
  });

  it('retire les copies gardées, une par carte unique', () => {
    // 10×0 + 5×1 + 2×2 = 9
    expect(eligibleCopies(rarity, 1)).toBe(9);
  });

  it('ne propose rien quand on garde plus que le plus gros empilement', () => {
    expect(eligibleCopies(rarity, 3)).toBe(0);
  });

  it('pondère par la part des binders retenus', () => {
    expect(eligibleCopies(rarity, 1, 0.5)).toBe(5); // arrondi de 4.5
  });
});

describe('binderFactor', () => {
  it('vaut 0 sans binder sélectionné', () => {
    expect(binderFactor('R', [])).toBe(0);
  });

  it('additionne les parts des binders connus sélectionnés', () => {
    expect(binderFactor('R', ['Trade Binder', 'Bulk'])).toBeCloseTo(0.45);
  });

  it('additionne le poids de chaque binder connu quand tous sont sélectionnés', () => {
    const all = Object.keys(BINDER_WEIGHTS);
    const expected = all.reduce((s, name) => s + BINDER_WEIGHTS[name]!.C, 0);
    expect(binderFactor('C', all)).toBeCloseTo(expected);
  });

  it('retombe sur le poids de repli pour un binder inconnu', () => {
    expect(binderFactor('R', ['Un binder ManaBox jamais vu'])).toBeCloseTo(FALLBACK_BINDER_WEIGHT);
  });

  it('borne le résultat à 1 même si la somme des poids le dépasse', () => {
    const all = Object.keys(BINDER_WEIGHTS);
    expect(binderFactor('C', [...all, 'Encore un binder inconnu'])).toBe(1);
  });
});
