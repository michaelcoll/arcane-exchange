import { describe, it, expect } from 'vitest';
import { computeDeal } from './deal';

describe('computeDeal', () => {
  it('reports a gain since purchase as a good deal', () => {
    expect(computeDeal(980, 1240)).toEqual({
      pct: 27,
      kind: 'good',
      abs: 27,
      sign: '+',
      deltaCents: 260,
    });
  });

  it('reports a loss since purchase as a bad deal, with a real minus sign', () => {
    expect(computeDeal(1000, 900)).toEqual({
      pct: -10,
      kind: 'bad',
      abs: 10,
      sign: '−',
      deltaCents: -100,
    });
  });

  it('treats a move under 3 % as par', () => {
    expect(computeDeal(1000, 1020)?.kind).toBe('par');
    expect(computeDeal(1000, 980)?.kind).toBe('par');
  });

  it('has no deal without a purchase price or a trend', () => {
    expect(computeDeal(0, 1000)).toBeNull();
    expect(computeDeal(undefined, 1000)).toBeNull();
    expect(computeDeal(1000, undefined)).toBeNull();
  });
});
