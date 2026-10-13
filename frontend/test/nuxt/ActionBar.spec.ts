import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import ActionBar from '~/components/Trade/ActionBar.vue';
import type { TradePrimaryAction } from '~/composables/useTradeView';

const accept: TradePrimaryAction = { kind: 'accept', label: 'Accepter et payer 21 €' };

const baseProps = { action: accept, partner: 'mizzix_42', busy: false };

describe('ActionBar', () => {
  it('accept: shows the label it received and emits "accept"', async () => {
    const wrapper = mount(ActionBar, { props: baseProps });
    const button = wrapper.get('button');
    expect(button.text()).toBe('Accepter et payer 21 €');
    await button.trigger('click');
    expect(wrapper.emitted('accept')).toHaveLength(1);
  });

  it('awaitingAcceptance: waits for the partner', () => {
    const wrapper = mount(ActionBar, {
      props: { ...baseProps, action: { kind: 'awaitingAcceptance' } },
    });
    expect(wrapper.find('button').exists()).toBe(false);
    expect(wrapper.text()).toBe('En attente de @mizzix_42');
  });

  it('confirmExchange: offers to confirm the exchange in secondary and emits "confirm"', async () => {
    const wrapper = mount(ActionBar, {
      props: { ...baseProps, action: { kind: 'confirmExchange' } },
    });
    const button = wrapper.get('button');
    expect(button.text()).toBe('Confirmer « échange réalisé »');
    expect(button.classes()).toContain('bg-secondary');
    await button.trigger('click');
    expect(wrapper.emitted('confirm')).toHaveLength(1);
  });

  it('awaitingConfirmation: waits for the partner', () => {
    const wrapper = mount(ActionBar, {
      props: { ...baseProps, action: { kind: 'awaitingConfirmation' } },
    });
    expect(wrapper.find('button').exists()).toBe(false);
    expect(wrapper.text()).toBe('Confirmé, en attente de @mizzix_42');
  });

  it('none: renders no bar at all', () => {
    const wrapper = mount(ActionBar, { props: { ...baseProps, action: { kind: 'none' } } });
    expect(wrapper.find('[data-part="bar"]').exists()).toBe(false);
  });

  it.each([accept, { kind: 'confirmExchange' }] satisfies TradePrimaryAction[])(
    'disables the $kind button while busy',
    (action) => {
      const wrapper = mount(ActionBar, { props: { ...baseProps, action, busy: true } });
      expect(wrapper.get('button').attributes('disabled')).toBeDefined();
    },
  );
});
