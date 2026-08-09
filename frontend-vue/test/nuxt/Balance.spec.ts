import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import Balance from '~/components/Trade/Balance.vue';

describe('Balance', () => {
  it('is balanced within the eur tolerance (< 3)', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 2, giveTotal: 50, getTotal: 52, mode: 'eur' },
    });
    expect(wrapper.text()).toContain('Équilibré');
  });

  it('is unbalanced at the eur tolerance boundary (3)', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 3, giveTotal: 50, getTotal: 53, mode: 'eur' },
    });
    expect(wrapper.text()).toContain('Tu dois €3');
  });

  it('shows what the current user owes when diff is positive (eur)', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 10, giveTotal: 60, getTotal: 50, mode: 'eur' },
    });
    expect(wrapper.text()).toContain('Tu dois €10');
  });

  it('shows what is owed to the current user when diff is negative (eur)', async () => {
    const wrapper = mount(Balance, {
      props: { diff: -10, giveTotal: 50, getTotal: 60, mode: 'eur' },
    });
    expect(wrapper.text()).toContain('On te doit €10');
  });

  it('is balanced within the edh tolerance (< 2)', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 1, giveTotal: 30, getTotal: 31, mode: 'edh' },
    });
    expect(wrapper.text()).toContain('≈ équilibré');
  });

  it('shows the edh advantage for the current user', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 5, giveTotal: 40, getTotal: 35, mode: 'edh' },
    });
    expect(wrapper.text()).toContain('À ton avantage · +5 pts');
  });

  it('shows the edh advantage for the counterparty', async () => {
    const wrapper = mount(Balance, {
      props: { diff: -5, giveTotal: 35, getTotal: 40, mode: 'edh' },
    });
    expect(wrapper.text()).toContain('À son avantage · +5 pts');
  });

  it('formats totals as euros in eur mode', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 0, giveTotal: 50, getTotal: 50, mode: 'eur' },
    });
    expect(wrapper.text()).toContain('€50');
  });

  it('formats totals as percentages in edh mode', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 0, giveTotal: 20, getTotal: 20, mode: 'edh' },
    });
    expect(wrapper.text()).toContain('20%');
  });

  it('does not divide by zero when both totals are 0', async () => {
    const wrapper = mount(Balance, {
      props: { diff: 0, giveTotal: 0, getTotal: 0, mode: 'eur' },
    });
    expect(wrapper.text()).toContain('Équilibré');
  });
});
