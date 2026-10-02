export type PriceFreshness = { label: string; tone: 'good' | 'muted' };

/* Prices are dated to the day (`YYYY-MM-DD`): the gap is counted in calendar days, between local
 * midnights. Rounding absorbs the 23- or 25-hour day of a DST change. */
export const priceFreshness = (lastPriceDate: string, now = new Date()): PriceFreshness => {
  const today = new Date(now).setHours(0, 0, 0, 0);
  const priced = new Date(`${lastPriceDate}T00:00`).getTime();
  const days = Math.max(0, Math.round((today - priced) / 86_400_000));

  const when = days === 0 ? "aujourd'hui" : days === 1 ? 'hier' : `il y a ${days} jours`;
  return { label: `Prix mis à jour ${when}`, tone: days <= 1 ? 'good' : 'muted' };
};
