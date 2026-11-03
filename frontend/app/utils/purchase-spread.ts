export type PurchaseSpreadKind = 'gain' | 'loss' | 'par';

/**
 * Spread between the purchase price and the trend price, in cents. Under 3 % in either direction,
 * the spread is considered neutral (`par`). `null` without a purchase price or without a trend price.
 */
export const computePurchaseSpread = (
  purchasedCents?: number | null,
  trendCents?: number | null,
) => {
  if (!purchasedCents || purchasedCents <= 0 || trendCents == null) return null;
  const deltaCents = trendCents - purchasedCents;
  const pct = Math.round((deltaCents / purchasedCents) * 100);
  const kind: PurchaseSpreadKind = pct >= 3 ? 'gain' : pct <= -3 ? 'loss' : 'par';
  return { pct, kind, abs: Math.abs(pct), sign: pct <= 0 ? '−' : '+', deltaCents };
};
