import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import ActionPanel from '~/components/Trade/ActionPanel.vue';

/** What the panel reads from the view of the Trade. */
type PanelView = InstanceType<typeof ActionPanel>['$props']['view'];

/** A view in negotiation, which I have not accepted yet. */
const view = (overrides: Partial<PanelView> = {}): PanelView => ({
  partnerUsername: 'mizzix_42',
  editable: true,
  reserved: false,
  abandonable: true,
  primaryAction: { kind: 'accept', label: 'Accepter et payer 21 €' },
  rating: { visible: false, canRate: false, mine: null, partner: null },
  busy: false,
  ...overrides,
});

const mountPanel = (overrides: Partial<PanelView> = {}) =>
  mount(ActionPanel, { props: { view: view(overrides) } });

const button = (wrapper: ReturnType<typeof mountPanel>, text: string) =>
  wrapper.findAll('button').find((b) => b.text() === text);

describe('ActionPanel', () => {
  it('accept: shows the label of the view, settlement included, and emits "accept"', async () => {
    const wrapper = mountPanel();
    await button(wrapper, 'Accepter et payer 21 €')!.trigger('click');
    expect(wrapper.emitted('accept')).toHaveLength(1);
  });

  it('awaitingAcceptance: waits for the partner with the words of the mobile bar', () => {
    const wrapper = mountPanel({ primaryAction: { kind: 'awaitingAcceptance' } });
    expect(wrapper.get('[data-part="action"]').text()).toBe('En attente de @mizzix_42');
    expect(wrapper.find('[data-part="action"] button').exists()).toBe(false);
  });

  it('confirmExchange: offers to confirm the exchange and emits "confirm"', async () => {
    const wrapper = mountPanel({ primaryAction: { kind: 'confirmExchange' } });
    await button(wrapper, 'Confirmer « échange réalisé »')!.trigger('click');
    expect(wrapper.emitted('confirm')).toHaveLength(1);
  });

  it('awaitingConfirmation: waits for the partner with the words of the mobile bar', () => {
    const wrapper = mountPanel({ primaryAction: { kind: 'awaitingConfirmation' } });
    expect(wrapper.get('[data-part="action"]').text()).toBe('Confirmé, en attente de @mizzix_42');
    expect(wrapper.find('[data-part="action"] button').exists()).toBe(false);
  });

  it('none: shows no primary action', () => {
    const wrapper = mountPanel({ primaryAction: { kind: 'none' }, abandonable: false });
    expect(wrapper.find('[data-part="action"]').exists()).toBe(false);
    expect(wrapper.find('button').exists()).toBe(false);
  });

  it('offers to abandon when the view says the Trade is abandonable, and emits "abandon"', async () => {
    const wrapper = mountPanel({ abandonable: true });
    await button(wrapper, 'Abandonner')!.trigger('click');
    expect(wrapper.emitted('abandon')).toHaveLength(1);
  });

  it('does not offer to abandon when the view says the Trade is not abandonable', () => {
    const wrapper = mountPanel({ abandonable: false });
    expect(button(wrapper, 'Abandonner')).toBeUndefined();
  });

  it('disables every button while busy', () => {
    const wrapper = mountPanel({ busy: true });
    const buttons = wrapper.findAll('button');
    expect(buttons).toHaveLength(2);
    for (const b of buttons) expect(b.attributes('disabled')).toBeDefined();
  });

  it.each([
    [
      'an editable Trade without reservation',
      { editable: true, reserved: false },
      'Modifiable par les deux parties · aucune carte réservée',
    ],
    [
      'an editable Trade with reserved cards',
      { editable: true, reserved: true },
      'Cartes réservées · modifiable (repasse en négociation)',
    ],
    [
      'an exchange to confirm',
      { editable: false, reserved: true, primaryAction: { kind: 'confirmExchange' } },
      'Échange physique en personne · confirmez chacun une fois réalisé',
    ],
    [
      'an exchange I confirmed',
      { editable: false, reserved: true, primaryAction: { kind: 'awaitingConfirmation' } },
      'Échange physique en personne · confirmez chacun une fois réalisé',
    ],
    [
      'a Trade with nothing left to do',
      { editable: false, abandonable: false, primaryAction: { kind: 'none' } },
      'Cette transaction est close.',
    ],
  ] satisfies [string, Partial<PanelView>, string][])('reminds the rule of %s', (_, o, hint) => {
    expect(mountPanel(o).get('[data-part="hint"]').text()).toBe(hint);
  });

  describe('rating', () => {
    const rated = (rating: Partial<PanelView['rating']>): Partial<PanelView> => ({
      editable: false,
      reserved: false,
      abandonable: false,
      primaryAction: { kind: 'none' },
      rating: { visible: true, canRate: false, mine: null, partner: null, ...rating },
    });

    it('lets me rate or skip while the view says I can, with the words of mobile', async () => {
      const wrapper = mountPanel(rated({ canRate: true }));
      expect(wrapper.text()).toContain('Noter @mizzix_42');
      expect(wrapper.text()).toContain(
        "Optionnel. L'échange se clôture dès que vous avez tous les deux noté ou passé.",
      );
      await wrapper.findAll('button[aria-label^="Noter "]')[3]!.trigger('click');
      await button(wrapper, 'Passer la notation')!.trigger('click');
      expect(wrapper.emitted('rate')).toEqual([[4]]);
      expect(wrapper.emitted('skip')).toHaveLength(1);
    });

    it('shows both ratings once given, the one my partner gave me included', () => {
      const wrapper = mountPanel(rated({ mine: 5, partner: 4 }));
      expect(wrapper.text()).toContain('Tu as mis 5/5');
      expect(wrapper.text()).toContain("@mizzix_42 t'a mis 4/5.");
      expect(wrapper.find('button').exists()).toBe(false);
    });

    it('reads a skipped rating as « Notation passée » on both sides', () => {
      const wrapper = mountPanel(rated({ mine: 0, partner: 0 }));
      expect(wrapper.text()).toContain('Notation passée');
      expect(wrapper.text()).toContain('@mizzix_42 a passé la notation.');
    });

    it('replaces the action row: no hint, no action', () => {
      const wrapper = mountPanel(rated({ mine: 5 }));
      expect(wrapper.find('[data-part="hint"]').exists()).toBe(false);
      expect(wrapper.find('[data-part="action"]').exists()).toBe(false);
    });

    it('shows no rating when the view hides it', () => {
      expect(mountPanel().text()).not.toContain('Noter');
    });
  });
});
