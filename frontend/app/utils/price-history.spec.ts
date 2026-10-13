import { describe, it, expect, vi, afterEach } from 'vitest';
import type { PriceHistoryEntry } from '~/bindings/PriceHistoryEntry';
import { priceTrend, periodDateRange, dayTicks, nearestIndex, formatDay } from './price-history';

const entry = (overrides: Partial<PriceHistoryEntry> = {}): PriceHistoryEntry => ({
  date: '2026-01-15',
  low: 1000,
  avg: 1200,
  trend: 1500,
  ...overrides,
});

const dayLabelFormatter = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' });
const labelFor = (year: number, month: number, day: number) =>
  dayLabelFormatter.format(new Date(year, month - 1, day));

describe('priceTrend', () => {
  it('converts cent amounts to euros', () => {
    const [point] = priceTrend([entry({ low: 1000, avg: 1200, trend: 1500 })]).points;
    expect(point).toMatchObject({ low: 10, avg: 12, trend: 15 });
  });

  it('formats the date of a point into a localized day/month label', () => {
    const [point] = priceTrend([entry({ date: '2026-03-05' })]).points;
    expect(point!.label).toBe(labelFor(2026, 3, 5));
  });

  it('keeps the points in the order of the entries', () => {
    const { points } = priceTrend([
      entry({ date: '2026-01-01', trend: 100 }),
      entry({ date: '2026-01-02', trend: 200 }),
    ]);
    expect(points.map((p) => p.trend)).toEqual([1, 2]);
  });

  it('numbers the days so that gaps between entries keep their real length', () => {
    const { points } = priceTrend([
      entry({ date: '2026-09-12' }),
      entry({ date: '2026-09-25' }),
      entry({ date: '2026-09-27' }),
    ]);
    const [a, b, c] = points.map((p) => p.day);
    expect(b! - a!).toBe(13);
    expect(c! - b!).toBe(2);
  });

  it('counts a DST change as a single day', () => {
    const [a, b] = priceTrend([
      entry({ date: '2026-10-24' }),
      entry({ date: '2026-10-26' }),
    ]).points;
    expect(b!.day - a!.day).toBe(2);
  });

  it('has no point for an empty history', () => {
    expect(priceTrend([]).points).toEqual([]);
  });

  it('has enough history from two points on', () => {
    expect(priceTrend([]).hasEnoughHistory).toBe(false);
    expect(priceTrend([entry()]).hasEnoughHistory).toBe(false);
    expect(priceTrend([entry(), entry({ date: '2026-01-16' })]).hasEnoughHistory).toBe(true);
  });

  it('reports the trend of the last entry, in cents', () => {
    const trend = priceTrend([entry({ trend: 1000 }), entry({ trend: 1234 })]);
    expect(trend.lastTrendCents).toBe(1234);
  });

  it('reports a last trend of zero for an empty history', () => {
    expect(priceTrend([]).lastTrendCents).toBe(0);
  });

  it('reports the variation between the first and the last trend', () => {
    const { variation } = priceTrend([
      entry({ trend: 1000 }),
      entry({ trend: 1100 }),
      entry({ trend: 1200 }),
    ]);
    expect(variation).toEqual({ positive: true, pctLabel: '+20,0 %', deltaCents: 200 });
  });

  it('flags a decrease as negative, with a true minus sign', () => {
    const { variation } = priceTrend([entry({ trend: 1000 }), entry({ trend: 800 })]);
    expect(variation).toEqual({ positive: false, pctLabel: '−20,0 %', deltaCents: -200 });
  });

  it('treats a zero variation as positive', () => {
    const { variation } = priceTrend([entry({ trend: 1000 }), entry({ trend: 1000 })]);
    expect(variation).toEqual({ positive: true, pctLabel: '+0,0 %', deltaCents: 0 });
  });

  it('reports a flat, positive variation for fewer than two entries', () => {
    const flat = { positive: true, pctLabel: '+0,0 %', deltaCents: 0 };
    expect(priceTrend([]).variation).toEqual(flat);
    expect(priceTrend([entry()]).variation).toEqual(flat);
  });

  it('reports 0 % but keeps the cent delta when the first trend is zero', () => {
    const { variation } = priceTrend([entry({ trend: 0 }), entry({ trend: 500 })]);
    expect(variation).toEqual({ positive: true, pctLabel: '+0,0 %', deltaCents: 500 });
  });

  it('writes the percentage with a decimal comma and exactly one decimal', () => {
    const { variation } = priceTrend([entry({ trend: 100000 }), entry({ trend: 101200 })]);
    expect(variation.pctLabel).toBe('+1,2 %');
  });

  it('rounds the percentage to one decimal', () => {
    const up = priceTrend([entry({ trend: 10000 }), entry({ trend: 10126 })]);
    expect(up.variation.pctLabel).toBe('+1,3 %');
    const down = priceTrend([entry({ trend: 10000 }), entry({ trend: 9876 })]);
    expect(down.variation.pctLabel).toBe('−1,2 %');
  });
});

describe('periodDateRange', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("spans 30 days inclusive of today for '30 j'", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 2, 15));
    expect(periodDateRange('30 j')).toEqual({ start_date: '2026-02-14', end_date: '2026-03-15' });
  });

  it('zero-pads single-digit months and days', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 3));
    expect(periodDateRange('30 j')).toEqual({ start_date: '2026-05-05', end_date: '2026-06-03' });
  });

  it('crosses the year boundary', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 0, 5));
    expect(periodDateRange('30 j')).toEqual({ start_date: '2025-12-07', end_date: '2026-01-05' });
  });

  it("starts '30 j' in the previous month when today is the first of a month", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 1));
    expect(periodDateRange('30 j')).toEqual({ start_date: '2026-05-03', end_date: '2026-06-01' });
  });

  it("goes back 3 calendar months for '3 m'", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 9, 13));
    expect(periodDateRange('3 m')).toEqual({ start_date: '2026-07-13', end_date: '2026-10-13' });
  });

  it("goes back 1 calendar year for '1 an'", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 9, 13));
    expect(periodDateRange('1 an')).toEqual({ start_date: '2025-10-13', end_date: '2026-10-13' });
  });

  it("goes back 10 years for 'Max'", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 9, 13));
    expect(periodDateRange('Max')).toEqual({ start_date: '2016-10-13', end_date: '2026-10-13' });
  });

  it("crosses the year boundary for '3 m'", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 0, 5));
    expect(periodDateRange('3 m')).toEqual({ start_date: '2025-10-05', end_date: '2026-01-05' });
  });

  it("rolls over to the next month when '3 m' starts on a day its month does not have", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 4, 31));
    expect(periodDateRange('3 m')).toEqual({ start_date: '2026-03-03', end_date: '2026-05-31' });
  });

  it("starts '1 an' on March 1st when today is February 29th", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2028, 1, 29));
    expect(periodDateRange('1 an')).toEqual({ start_date: '2027-03-01', end_date: '2028-02-29' });
  });
});

describe('formatDay', () => {
  it('labels a day number like its entry', () => {
    const [point] = priceTrend([entry({ date: '2026-03-05' })]).points;
    expect(formatDay(point!.day)).toBe(labelFor(2026, 3, 5));
  });
});

describe('dayTicks', () => {
  it('spreads ticks evenly in time from the first to the last day', () => {
    expect(dayTicks(100, 130, 6)).toEqual([100, 106, 112, 118, 124, 130]);
  });

  it('rounds ticks to whole days', () => {
    expect(dayTicks(0, 7, 3)).toEqual([0, 4, 7]);
  });

  it('never repeats a day when the span is shorter than the tick count', () => {
    expect(dayTicks(10, 12, 6)).toEqual([10, 11, 12]);
  });

  it('returns a single tick for a single day', () => {
    expect(dayTicks(10, 10, 6)).toEqual([10]);
  });
});

describe('nearestIndex', () => {
  const days = [0, 3, 16, 18];

  it('picks the entry closest in time, not by position', () => {
    expect(nearestIndex(days, 8)).toBe(1);
    expect(nearestIndex(days, 12)).toBe(2);
  });

  it('clamps outside the range', () => {
    expect(nearestIndex(days, -5)).toBe(0);
    expect(nearestIndex(days, 40)).toBe(3);
  });
});
