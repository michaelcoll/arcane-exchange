import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import Column from '~/components/Trade/Column.vue';
import type { TradeCard } from '~/utils/trade';

const cards: TradeCard[] = [
  { name: 'Sol Ring', eur: 12, edh: 45 },
  { name: 'Vampiric Tutor', eur: 20, edh: 10 },
];

const baseProps = {
  label: 'Je donne',
  cards,
  mode: 'eur' as const,
  accent: 'neutral' as const,
  editable: true,
  reserved: false,
  addLabel: 'Ajouter une carte',
};

describe('Column', () => {
  it('lists every card with its formatted value', async () => {
    const wrapper = mount(Column, { props: baseProps });
    expect(wrapper.text()).toContain('Sol Ring');
    expect(wrapper.text()).toContain('€12');
    expect(wrapper.text()).toContain('Vampiric Tutor');
    expect(wrapper.text()).toContain('€20');
  });

  it('sums the total in eur mode', async () => {
    const wrapper = mount(Column, { props: baseProps });
    expect(wrapper.text()).toContain('€32');
  });

  it('sums the total in edh mode', async () => {
    const wrapper = mount(Column, { props: { ...baseProps, mode: 'edh' } });
    expect(wrapper.text()).toContain('55%');
  });

  it('shows the empty-state message when there are no cards', async () => {
    const wrapper = mount(Column, { props: { ...baseProps, cards: [] } });
    expect(wrapper.text()).toContain('Aucune carte de ce côté.');
  });

  it('pluralizes the card count', async () => {
    const one = mount(Column, { props: { ...baseProps, cards: [cards[0]!] } });
    expect(one.text()).toContain('1 carte');
    expect(one.text()).not.toContain('1 cartes');

    const many = mount(Column, { props: baseProps });
    expect(many.text()).toContain('2 cartes');
  });

  it('emits "remove" with the card index when editable', async () => {
    const wrapper = mount(Column, { props: baseProps });
    const removeButtons = wrapper.findAll('button[aria-label="Retirer la carte de l’échange"]');
    expect(removeButtons).toHaveLength(2);
    await removeButtons[1]!.trigger('click');
    expect(wrapper.emitted('remove')).toEqual([[1]]);
  });

  it('emits "add" when the add-card button is clicked', async () => {
    const wrapper = mount(Column, { props: baseProps });
    await wrapper.find('button:not([aria-label])').trigger('click');
    expect(wrapper.emitted('add')).toHaveLength(1);
  });

  it('hides remove buttons and the add button when not editable', async () => {
    const wrapper = mount(Column, { props: { ...baseProps, editable: false } });
    expect(wrapper.find('button[aria-label="Retirer la carte de l’échange"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('Ajouter une carte');
  });

  it('shows a "Réservée" tag per card when reserved', async () => {
    const wrapper = mount(Column, {
      props: { ...baseProps, editable: false, reserved: true },
    });
    const tags = wrapper.text().match(/Réservée/g) ?? [];
    expect(tags).toHaveLength(cards.length);
  });
});
