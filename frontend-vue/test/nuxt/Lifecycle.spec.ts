import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import Lifecycle from '~/components/Trade/Lifecycle.vue';
import { TRADE_LIFECYCLE, type TradeStatus } from '~/utils/trade';

describe('Lifecycle', () => {
  it.each([
    ['PENDING', 0],
    ['ONE_ACCEPTED', 1],
    ['FULLY_ACCEPTED', 2],
    ['COMPLETED', 3],
    ['CLOSED', 4],
  ] satisfies [TradeStatus, number][])(
    'marks %s with %i completed steps',
    async (status, doneCount) => {
      const wrapper = mount(Lifecycle, { props: { status } });
      expect(wrapper.findAll('.iconify')).toHaveLength(doneCount);
    },
  );

  it('renders every lifecycle step label', async () => {
    const wrapper = mount(Lifecycle, { props: { status: 'PENDING' } });
    for (const step of TRADE_LIFECYCLE) {
      expect(wrapper.text()).toContain(step.label);
    }
  });

  it('shows no completed step and dims the stepper when abandoned', async () => {
    const wrapper = mount(Lifecycle, { props: { status: 'ABANDONED' } });
    expect(wrapper.findAll('.iconify')).toHaveLength(0);
    expect(wrapper.classes().join(' ')).toContain('opacity-45');
  });

  it('does not dim the stepper on the nominal path', async () => {
    const wrapper = mount(Lifecycle, { props: { status: 'PENDING' } });
    expect(wrapper.classes().join(' ')).not.toContain('opacity-45');
  });
});
