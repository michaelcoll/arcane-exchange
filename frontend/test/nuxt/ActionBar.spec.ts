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

  describe('desktop layout', () => {
    const abandon = 'button[aria-label="Abandonner l\'échange"]';
    const desktop = { ...baseProps, layout: 'inline' as const, abandonable: true };

    it('offers to abandon the Trade with a × that emits "abandon"', async () => {
      const wrapper = mount(ActionBar, { props: desktop });
      await wrapper.get(abandon).trigger('click');
      expect(wrapper.emitted('abandon')).toHaveLength(1);
      expect(wrapper.emitted('accept')).toBeUndefined();
    });

    it('keeps the primary action the view chose', async () => {
      const wrapper = mount(ActionBar, { props: desktop });
      const accept = wrapper.findAll('button').find((b) => b.text() === 'Accepter et payer 21 €');
      await accept?.trigger('click');
      expect(wrapper.emitted('accept')).toHaveLength(1);
    });

    it('has no × once the Trade can no longer be abandoned', () => {
      const wrapper = mount(ActionBar, { props: { ...desktop, abandonable: false } });
      expect(wrapper.find(abandon).exists()).toBe(false);
    });

    it('disables every button while busy', () => {
      const wrapper = mount(ActionBar, { props: { ...desktop, busy: true } });
      const buttons = wrapper.findAll('button');
      expect(buttons).toHaveLength(2);
      for (const button of buttons) expect(button.attributes('disabled')).toBeDefined();
    });

    it('is not stuck to the bottom of the screen', () => {
      const wrapper = mount(ActionBar, { props: desktop });
      expect(wrapper.get('[data-part="bar"]').classes()).not.toContain('fixed');
    });
  });

  it('has no × on mobile, even when the Trade can be abandoned', () => {
    const wrapper = mount(ActionBar, { props: { ...baseProps, abandonable: true } });
    expect(wrapper.find('button[aria-label="Abandonner l\'échange"]').exists()).toBe(false);
    expect(wrapper.get('[data-part="bar"]').classes()).toContain('fixed');
  });
});
