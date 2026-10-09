export type DealKind = 'good' | 'bad' | 'par';

/**
 * Écart entre le prix d'achat et la tendance du marché, en centimes. Sous 3 % dans un sens ou
 * dans l'autre, l'écart est jugé neutre (`par`). `null` sans prix d'achat ou sans tendance.
 */
export const computeDeal = (purchasedCents?: number | null, trendCents?: number | null) => {
  if (!purchasedCents || purchasedCents <= 0 || trendCents == null) return null;
  const deltaCents = trendCents - purchasedCents;
  const pct = Math.round((deltaCents / purchasedCents) * 100);
  const kind: DealKind = pct >= 3 ? 'good' : pct <= -3 ? 'bad' : 'par';
  return { pct, kind, abs: Math.abs(pct), sign: pct <= 0 ? '−' : '+', deltaCents };
};
