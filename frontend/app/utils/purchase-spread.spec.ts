import { describe, it, expect } from 'vitest';
import { computePurchaseSpread } from './purchase-spread';

describe('computePurchaseSpread', () => {
  it('reports a rise since purchase as a gain', () => {
    expect(computePurchaseSpread(980, 1240)).toEqual({
      pct: 27,
      kind: 'gain',
      abs: 27,
      sign: '+',
      deltaCents: 260,
    });
  });

  it('reports a drop since purchase as a loss, with a real minus sign', () => {
    expect(computePurchaseSpread(1000, 900)).toEqual({
      pct: -10,
      kind: 'loss',
      abs: 10,
      sign: '−',
      deltaCents: -100,
    });
  });

  it('treats a move under 3 % as par', () => {
    expect(computePurchaseSpread(1000, 1020)?.kind).toBe('par');
    expect(computePurchaseSpread(1000, 980)?.kind).toBe('par');
  });

  it('has no spread without a purchase price or a trend', () => {
    expect(computePurchaseSpread(0, 1000)).toBeNull();
    expect(computePurchaseSpread(undefined, 1000)).toBeNull();
    expect(computePurchaseSpread(1000, undefined)).toBeNull();
  });
});
