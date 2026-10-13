import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import Balance from '~/components/Trade/Balance.vue';
import type { TradeViewBalance } from '~/composables/useTradeView';
import formatPrice from '~/utils/format-price';

const owing: TradeViewBalance = { delta: 1000, balanced: false, settlement: 'payer 10 €' };

const verdict = (balance: TradeViewBalance) =>
  mount(Balance, { props: { balance, giveTotal: 5000, getTotal: 5000 + balance.delta } })
    .get('[data-part="verdict"]')
    .text();

describe('Balance', () => {
  it('reads « Équilibré » when the view says the Trade is balanced, whatever the delta', () => {
    expect(verdict({ delta: 200, balanced: true, settlement: null })).toBe('Équilibré');
  });

  it('does not decide the balance itself: a small delta the view calls unbalanced is owed', () => {
    expect(verdict({ delta: 100, balanced: false, settlement: 'payer 1 €' })).toBe(
      `Tu dois ${formatPrice(100)}`,
    );
  });

  it('shows what I owe when the delta is positive', () => {
    expect(verdict(owing)).toBe(`Tu dois ${formatPrice(1000)}`);
  });

  it('shows what I am owed when the delta is negative', () => {
    expect(verdict({ delta: -1000, balanced: false, settlement: 'recevoir 10 €' })).toBe(
      `On te doit ${formatPrice(1000)}`,
    );
  });

  it('formats give/get totals as prices', () => {
    const wrapper = mount(Balance, { props: { balance: owing, giveTotal: 4000, getTotal: 5000 } });
    expect(wrapper.get('[data-side="give"][data-part="total"]').text()).toBe(formatPrice(4000));
    expect(wrapper.get('[data-side="get"][data-part="total"]').text()).toBe(formatPrice(5000));
  });

  describe('roles', () => {
    const ROLE_CLASS = /\b(?:text|bg)-(?:primary|secondary)\b/;
    const mountBalance = () =>
      mount(Balance, { props: { balance: owing, giveTotal: 4000, getTotal: 5000 } });

    it('keeps the « give » side neutral (label, bar segment, total)', () => {
      const wrapper = mountBalance();
      for (const part of ['label', 'bar', 'total']) {
        const el = wrapper.get(`[data-side="give"][data-part="${part}"]`);
        expect(el.attributes('class')).not.toMatch(ROLE_CLASS);
      }
    });

    it('shows the « get » side in secondary (label, bar segment, total)', () => {
      const wrapper = mountBalance();
      expect(wrapper.get('[data-side="get"][data-part="label"]').classes()).toContain(
        'text-secondary',
      );
      expect(wrapper.get('[data-side="get"][data-part="bar"]').classes()).toContain('bg-secondary');
      expect(wrapper.get('[data-side="get"][data-part="total"]').classes()).toContain(
        'text-secondary',
      );
    });

    it('keeps the verdict in secondary when there is a cash delta', () => {
      const wrapper = mountBalance();
      expect(wrapper.get('[data-part="verdict"]').classes()).toContain('text-secondary-ink');
    });
  });

  it('does not divide by zero when both totals are 0', () => {
    const wrapper = mount(Balance, {
      props: { balance: { delta: 0, balanced: true, settlement: null }, giveTotal: 0, getTotal: 0 },
    });
    expect(wrapper.get('[data-side="get"][data-part="bar"]').attributes('style')).toContain(
      'width: 0%',
    );
  });
});
