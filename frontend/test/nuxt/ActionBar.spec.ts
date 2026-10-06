import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import ActionBar from '~/components/Trade/ActionBar.vue';
import type { TradeStatus } from '~/utils/trade';
import formatPrice from '~/utils/format-price';

const baseProps = {
  status: 'PENDING' as TradeStatus,
  meAccepted: false,
  meConfirmed: false,
  partner: 'mizzix_42',
  diff: 0,
  busy: false,
};

describe('ActionBar', () => {
  it.each(['PENDING', 'ONE_ACCEPTED'] satisfies TradeStatus[])(
    '%s, not yet accepted by me: offers to accept and emits "accept"',
    async (status) => {
      const wrapper = mount(ActionBar, { props: { ...baseProps, status } });
      const button = wrapper.get('button');
      expect(button.text()).toBe("Accepter l'échange");
      await button.trigger('click');
      expect(wrapper.emitted('accept')).toHaveLength(1);
    },
  );

  it('puts what I pay on the accept button', () => {
    const wrapper = mount(ActionBar, { props: { ...baseProps, diff: 2100 } });
    expect(wrapper.get('button').text()).toBe(`Accepter et payer ${formatPrice(2100)}`);
  });

  it('puts what I receive on the accept button', () => {
    const wrapper = mount(ActionBar, { props: { ...baseProps, diff: -400 } });
    expect(wrapper.get('button').text()).toBe(`Accepter et recevoir ${formatPrice(400)}`);
  });

  it.each(['PENDING', 'ONE_ACCEPTED'] satisfies TradeStatus[])(
    '%s, already accepted by me: waits for the partner',
    (status) => {
      const wrapper = mount(ActionBar, { props: { ...baseProps, status, meAccepted: true } });
      expect(wrapper.find('button').exists()).toBe(false);
      expect(wrapper.text()).toBe('En attente de @mizzix_42');
    },
  );

  it('FULLY_ACCEPTED: offers to confirm the exchange in secondary and emits "confirm"', async () => {
    const wrapper = mount(ActionBar, {
      props: { ...baseProps, status: 'FULLY_ACCEPTED', meAccepted: true },
    });
    const button = wrapper.get('button');
    expect(button.text()).toBe('Confirmer « échange réalisé »');
    expect(button.classes()).toContain('bg-secondary');
    await button.trigger('click');
    expect(wrapper.emitted('confirm')).toHaveLength(1);
  });

  it('FULLY_ACCEPTED, confirmed by me: waits for the partner', () => {
    const wrapper = mount(ActionBar, {
      props: { ...baseProps, status: 'FULLY_ACCEPTED', meAccepted: true, meConfirmed: true },
    });
    expect(wrapper.find('button').exists()).toBe(false);
    expect(wrapper.text()).toBe('Confirmé, en attente de @mizzix_42');
  });

  it.each(['COMPLETED', 'CLOSED', 'ABANDONED'] satisfies TradeStatus[])(
    '%s: renders no bar at all',
    (status) => {
      const wrapper = mount(ActionBar, { props: { ...baseProps, status, meAccepted: true } });
      expect(wrapper.find('[data-part="bar"]').exists()).toBe(false);
    },
  );

  it('disables the action while busy', () => {
    const wrapper = mount(ActionBar, { props: { ...baseProps, busy: true } });
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
  });
});
