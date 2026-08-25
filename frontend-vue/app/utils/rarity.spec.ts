import { describe, it, expect } from 'vitest';
import { RARITY_ICON_COLOR_CLASS, RARITY_ORDER } from './rarity';

describe('RARITY_ICON_COLOR_CLASS', () => {
  it('has an entry for every known rarity code', () => {
    for (const code of RARITY_ORDER) {
      expect(RARITY_ICON_COLOR_CLASS[code]).toBeDefined();
    }
  });

  it('assigns a distinct color class to each rarity', () => {
    const colors = RARITY_ORDER.map((code) => RARITY_ICON_COLOR_CLASS[code]);
    expect(new Set(colors).size).toBe(colors.length);
  });
});
