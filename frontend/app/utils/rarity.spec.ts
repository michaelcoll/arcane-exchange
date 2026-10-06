import { describe, it, expect } from 'vitest';
import { RARITY_COLOR_CLASS } from './rarity';

describe('RARITY_COLOR_CLASS', () => {
  it('maps each rarity to its semantic rarity text color', () => {
    expect(RARITY_COLOR_CLASS).toEqual({
      C: 'text-rarity-common',
      U: 'text-rarity-uncommon',
      R: 'text-rarity-rare',
      M: 'text-rarity-mythic',
      S: 'text-rarity-special',
    });
  });
});
