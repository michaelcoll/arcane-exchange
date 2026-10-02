export type PriceFreshness = { label: string; tone: 'good' | 'muted' };

const MS_PER_DAY = 86_400_000;

/* Prices are dated to the day (`YYYY-MM-DD`): the gap is counted in calendar days, in local time.
 * Each date is read as UTC midnight so a DST change cannot shave an hour off a day. */
export const priceFreshness = (lastPriceDate: string, now = new Date()): PriceFreshness => {
  const [year, month, day] = lastPriceDate.split('-').map(Number) as [number, number, number];
  const today = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate());
  const days = Math.max(0, Math.round((today - Date.UTC(year, month - 1, day)) / MS_PER_DAY));

  const when = days === 0 ? "aujourd'hui" : days === 1 ? 'hier' : `il y a ${days} jours`;
  return { label: `Prix mis à jour ${when}`, tone: days <= 1 ? 'good' : 'muted' };
};
