import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';

const dayLabelFormatter = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' });

export const toEnvelopeData = (entries: PriceHistoryEntry[]) =>
  entries.map((e) => {
    const [year, month, day] = e.date.split('-').map(Number);
    return {
      low: e.low / 100,
      avg: e.avg / 100,
      trend: e.trend / 100,
      label: dayLabelFormatter.format(new Date(year!, month! - 1, day)),
    };
  });

export const computeVariation = (entries: PriceHistoryEntry[]) => {
  if (entries.length < 2) return { pct: 0, deltaCents: 0, positive: true };
  const first = entries[0]!.trend;
  const last = entries[entries.length - 1]!.trend;
  const pct = first !== 0 ? ((last - first) / first) * 100 : 0;
  return { pct, deltaCents: last - first, positive: pct >= 0 };
};

const toIsoDate = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

export const lastNDaysRange = (days: number) => {
  const end = new Date();
  const start = new Date(end);
  start.setDate(start.getDate() - (days - 1));
  return { start_date: toIsoDate(start), end_date: toIsoDate(end) };
};
