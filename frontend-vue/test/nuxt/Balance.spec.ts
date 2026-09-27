import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import Balance from '~/components/Trade/Balance.vue';
import formatPrice from '~/utils/format-price';

describe('Balance', () => {
  it('is balanced within the tolerance (< 300 centimes)', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 200, giveTotal: 5000, getTotal: 5200 },
    });
    expect(wrapper.text()).toContain('Équilibré');
  });

  it('is unbalanced at the tolerance boundary (300 centimes)', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 300, giveTotal: 5000, getTotal: 5300 },
    });
    expect(wrapper.text()).toContain(`Tu dois ${formatPrice(300)}`);
  });

  it('shows what the current user owes when diff is positive', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 1000, giveTotal: 6000, getTotal: 5000 },
    });
    expect(wrapper.text()).toContain(`Tu dois ${formatPrice(1000)}`);
  });

  it('shows what is owed to the current user when diff is negative', async () => {
    const wrapper = mount(Balance, {
      props: { diff: -1000, giveTotal: 5000, getTotal: 6000 },
    });
    expect(wrapper.text()).toContain(`On te doit ${formatPrice(1000)}`);
  });

  it('formats give/get totals as prices', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 0, giveTotal: 5000, getTotal: 5000 },
    });
    expect(wrapper.text()).toContain(formatPrice(5000));
  });

  describe('roles', () => {
    const ROLE_CLASS = /\b(?:text|bg)-(?:primary|secondary)\b/;
    const mountBalance = () =>
      mount(Balance, { props: { diff: 1000, giveTotal: 4000, getTotal: 5000 } });

    it('keeps the « give » side neutral (label, bar segment, total)', async () => {
      const wrapper = mountBalance();
      for (const part of ['label', 'bar', 'total']) {
        const el = wrapper.get(`[data-side="give"][data-part="${part}"]`);
        expect(el.attributes('class')).not.toMatch(ROLE_CLASS);
      }
    });

    it('shows the « get » side in secondary (label, bar segment, total)', async () => {
      const wrapper = mountBalance();
      expect(wrapper.get('[data-side="get"][data-part="label"]').classes()).toContain(
        'text-secondary',
      );
      expect(wrapper.get('[data-side="get"][data-part="bar"]').classes()).toContain('bg-secondary');
      expect(wrapper.get('[data-side="get"][data-part="total"]').classes()).toContain(
        'text-secondary',
      );
    });

    it('keeps the verdict in secondary when there is a cash delta', async () => {
      const wrapper = mountBalance();
      expect(wrapper.get('[data-part="verdict"]').classes()).toContain('text-secondary-ink');
    });
  });

  it('does not divide by zero when both totals are 0', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 0, giveTotal: 0, getTotal: 0 },
    });
    expect(wrapper.text()).toContain('Équilibré');
  });
});
