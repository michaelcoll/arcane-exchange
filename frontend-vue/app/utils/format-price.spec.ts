import { describe, expect, it } from 'vitest';
import formatPrice, { formatChartPrice } from './format-price';

// Intl uses (narrow) no-break spaces; normalize them for readable assertions.
const plain = (s: string) => s.replace(/\s/g, ' ');

describe('formatPrice', () => {
  it('shows two decimals when the amount has cents', () => {
    expect(plain(formatPrice(350))).toBe('3,50 €');
    expect(plain(formatPrice(35))).toBe('0,35 €');
    expect(plain(formatPrice(4567))).toBe('45,67 €');
  });

  it('drops decimals for a whole amount', () => {
    expect(plain(formatPrice(1200))).toBe('12 €');
    expect(plain(formatPrice(210000))).toBe('2 100 €');
  });

  it('treats a missing amount as zero', () => {
    expect(plain(formatPrice())).toBe('0 €');
  });
});

describe('formatChartPrice', () => {
  it('keeps cents below 100 €', () => {
    expect(plain(formatChartPrice(0.3))).toBe('0,30 €');
    expect(plain(formatChartPrice(99.5))).toBe('99,50 €');
    expect(plain(formatChartPrice(12))).toBe('12 €');
  });

  it('rounds to the euro from 100 €', () => {
    expect(plain(formatChartPrice(100))).toBe('100 €');
    expect(plain(formatChartPrice(145.67))).toBe('146 €');
    expect(plain(formatChartPrice(2100.4))).toBe('2 100 €');
  });

  it('rounds sub-cent axis values to the nearest cent', () => {
    expect(plain(formatChartPrice(0.2345))).toBe('0,23 €');
    expect(plain(formatChartPrice(-0.042))).toBe('-0,04 €');
  });
});
