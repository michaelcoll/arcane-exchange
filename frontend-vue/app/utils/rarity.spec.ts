import { describe, it, expect } from 'vitest';
import { RARITY_COLOR_CLASS } from './rarity';

describe('RARITY_COLOR_CLASS', () => {
  it('maps each rarity to its semantic rarity color, for text and border', () => {
    expect(RARITY_COLOR_CLASS).toEqual({
      C: { text: 'text-rarity-common', border: 'border-rarity-common' },
      U: { text: 'text-rarity-uncommon', border: 'border-rarity-uncommon' },
      R: { text: 'text-rarity-rare', border: 'border-rarity-rare' },
      M: { text: 'text-rarity-mythic', border: 'border-rarity-mythic' },
      S: { text: 'text-rarity-special', border: 'border-rarity-special' },
    });
  });
});
