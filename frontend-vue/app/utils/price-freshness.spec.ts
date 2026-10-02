import { describe, it, expect } from 'vitest';
import { priceFreshness } from './price-freshness';

// Late evening, local time: the day must not drift with the offset from UTC.
const now = new Date(2026, 9, 2, 23, 30);

describe('priceFreshness', () => {
  it('says « aujourd’hui », in the good tone, for a price dated today', () => {
    expect(priceFreshness('2026-10-02', now)).toEqual({
      label: "Prix mis à jour aujourd'hui",
      tone: 'good',
    });
  });

  it('says « hier », still in the good tone, for a price dated yesterday', () => {
    expect(priceFreshness('2026-10-01', now)).toEqual({
      label: 'Prix mis à jour hier',
      tone: 'good',
    });
  });

  it('counts calendar days, in the muted tone, beyond one day', () => {
    expect(priceFreshness('2026-09-29', now)).toEqual({
      label: 'Prix mis à jour il y a 3 jours',
      tone: 'muted',
    });
  });

  it('counts calendar days across a month boundary', () => {
    expect(priceFreshness('2026-09-30', new Date(2026, 9, 1, 0, 5)).label).toBe(
      'Prix mis à jour hier',
    );
  });

  it('counts calendar days across a DST change', () => {
    expect(priceFreshness('2026-10-20', new Date(2026, 10, 3, 9, 0)).label).toBe(
      'Prix mis à jour il y a 14 jours',
    );
  });

  it('treats a date in the future as today', () => {
    expect(priceFreshness('2026-10-03', now).label).toBe("Prix mis à jour aujourd'hui");
  });
});
