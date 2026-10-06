import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import StatusPill from '~/components/Trade/StatusPill.vue';
import { TRADE_STATUS_META, type TradeStatus } from '~/utils/trade';

describe('StatusPill', () => {
  it.each(Object.keys(TRADE_STATUS_META) as TradeStatus[])(
    'renders the label for status %s',
    async (status) => {
      const wrapper = mount(StatusPill, { props: { status } });
      expect(wrapper.text()).toContain(TRADE_STATUS_META[status].label);
    },
  );

  it('defaults to the "md" size', async () => {
    const wrapper = mount(StatusPill, { props: { status: 'PENDING' } });
    expect(wrapper.find('span').classes().join(' ')).toContain('text-2xs');
  });

  it('applies the "sm" size classes when requested', async () => {
    const wrapper = mount(StatusPill, { props: { status: 'PENDING', size: 'sm' } });
    expect(wrapper.find('span').classes().join(' ')).toContain('text-[10px]');
  });
});
