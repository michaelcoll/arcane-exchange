import { describe, it, expect } from 'vitest';
import { isSameQuery } from '~/composables/useQuerySync';

describe('isSameQuery', () => {
  it('matches the same parameters', () => {
    expect(isSameQuery({ q: 'a', mode: 'name' }, { mode: 'name', q: 'a' })).toBe(true);
  });

  it.each([
    ['a missing parameter', { mode: 'name' }],
    ['an extra parameter', { mode: 'name', q: 'a', player: 'urza' }],
    ['a different value', { mode: 'name', q: 'b' }],
  ])('rejects %s', (_label, current) => {
    expect(isSameQuery(current, { mode: 'name', q: 'a' })).toBe(false);
  });
});
