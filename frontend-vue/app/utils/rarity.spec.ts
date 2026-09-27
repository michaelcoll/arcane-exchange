import { describe, it, expect } from 'vitest';
import { RARITY_COLOR_CLASS, RARITY_ORDER } from './rarity';

describe('RARITY_COLOR_CLASS', () => {
  it('has an entry for every known rarity code', () => {
    for (const code of RARITY_ORDER) {
      expect(RARITY_COLOR_CLASS[code]).toBeDefined();
    }
  });

  it('assigns a distinct color class to each rarity', () => {
    const colors = RARITY_ORDER.map((code) => RARITY_COLOR_CLASS[code]);
    expect(new Set(colors).size).toBe(colors.length);
  });

  it('maps each rarity to its semantic rarity color, never a neutral scale', () => {
    expect(RARITY_COLOR_CLASS).toEqual({
      C: 'text-rarity-common',
      U: 'text-rarity-uncommon',
      R: 'text-rarity-rare',
      M: 'text-rarity-mythic',
      S: 'text-rarity-special',
    });
  });
});
