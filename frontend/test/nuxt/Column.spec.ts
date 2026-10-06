import { describe, it, expect } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import Column from '~/components/Trade/Column.vue';
import type { TradeCard } from '~/bindings/TradeCard';
import formatPrice from '~/utils/format-price';

const card = (overrides: Partial<TradeCard> = {}): TradeCard => ({
  set_code: 'ECL',
  collector_number: '166',
  language_code: 'FR',
  foil: false,
  name: 'Sol Ring',
  quantity: 1,
  price_guide: { low: 800, avg: 1000, trend: 1200 },
  scryfall_id: 'sol-ring',
  image_url: null,
  image_back_url: null,
  ...overrides,
});

const cards: TradeCard[] = [
  card({
    name: 'Sol Ring',
    collector_number: '166',
    quantity: 1,
    price_guide: { low: 0, avg: 0, trend: 1200 },
  }),
  card({
    name: 'Vampiric Tutor',
    collector_number: '167',
    quantity: 2,
    price_guide: { low: 0, avg: 0, trend: 2000 },
  }),
];

const baseProps = {
  label: 'Je donne',
  cards,
  tone: 'neutral' as const,
  reserved: false,
  removable: true,
  addLabel: 'Ajouter une carte',
};

const ROLE_TEXT = /\btext-(?:primary|secondary)\b/;

/** The `<span>` whose whole text is `text` (a price, the column label…). */
const spanWithText = (wrapper: VueWrapper, text: string) =>
  wrapper.findAll('span').find((s) => s.text() === text)!;

const addButton = (wrapper: VueWrapper) => wrapper.get('button:not([aria-label])');

describe('Column', () => {
  it('lists every card with its formatted value', () => {
    const wrapper = mount(Column, { props: baseProps });
    expect(wrapper.text()).toContain('Sol Ring');
    expect(wrapper.text()).toContain(formatPrice(1200));
    expect(wrapper.text()).toContain('Vampiric Tutor');
    expect(wrapper.text()).toContain(formatPrice(4000));
  });

  it('shows ×N only when the quantity is greater than 1', () => {
    const wrapper = mount(Column, { props: baseProps });
    expect(wrapper.text()).not.toContain('×1');
    expect(wrapper.text()).toContain('×2');
  });

  it('sums the value of every line into the column total', () => {
    const wrapper = mount(Column, { props: baseProps });
    // 1200×1 + 2000×2 = 5200 centimes
    expect(wrapper.text()).toContain(formatPrice(5200));
  });

  it('shows the empty-state message when there are no cards', () => {
    const wrapper = mount(Column, { props: { ...baseProps, cards: [] } });
    expect(wrapper.text()).toContain('Aucune carte de ce côté.');
  });

  it('counts cards by summed quantity, not by line count', () => {
    const wrapper = mount(Column, { props: baseProps });
    expect(wrapper.text()).toContain('3 cartes');
  });

  it('pluralizes the card count for a single card', () => {
    const wrapper = mount(Column, {
      props: { ...baseProps, cards: [card({ quantity: 1 })] },
    });
    expect(wrapper.text()).toContain('1 carte');
    expect(wrapper.text()).not.toContain('1 cartes');
  });

  it('emits "remove" with the card object when removable', async () => {
    const wrapper = mount(Column, { props: baseProps });
    const removeButtons = wrapper.findAll('button[aria-label="Retirer la carte de l’échange"]');
    expect(removeButtons).toHaveLength(2);
    await removeButtons[1]!.trigger('click');
    expect(wrapper.emitted('remove')).toEqual([[cards[1]]]);
  });

  it('emits "add" when the add-card button is clicked', async () => {
    const wrapper = mount(Column, { props: baseProps });
    await addButton(wrapper).trigger('click');
    expect(wrapper.emitted('add')).toHaveLength(1);
  });

  it('does not render the add button when addLabel is not provided', () => {
    const wrapper = mount(Column, { props: { ...baseProps, addLabel: undefined } });
    expect(wrapper.text()).not.toContain('Ajouter une carte');
  });

  it('hides remove buttons when not removable', () => {
    const wrapper = mount(Column, { props: { ...baseProps, removable: false } });
    expect(wrapper.find('button[aria-label="Retirer la carte de l’échange"]').exists()).toBe(false);
  });

  describe('secondary tone (« Je reçois »)', () => {
    const mountSecondary = () =>
      mount(Column, { props: { ...baseProps, label: 'Je reçois', tone: 'secondary' as const } });

    it('shows the label, the card values and the total in secondary', () => {
      const wrapper = mountSecondary();
      expect(spanWithText(wrapper, 'Je reçois').classes()).toContain('text-secondary');
      expect(spanWithText(wrapper, formatPrice(1200)).classes()).toContain('text-secondary');
      expect(spanWithText(wrapper, formatPrice(5200)).classes()).toContain('text-secondary');
    });

    it('highlights the add button in secondary on hover and focus', () => {
      const classes = addButton(mountSecondary()).classes();
      expect(classes).toEqual(
        expect.arrayContaining([
          'hover:border-secondary/40',
          'hover:bg-secondary/10',
          'focus-visible:border-secondary/40',
          'focus-visible:bg-secondary/10',
        ]),
      );
      expect(classes.join(' ')).not.toMatch(/\bprimary\b/);
    });
  });

  describe('neutral tone (« Je donne »)', () => {
    it('shows the label and the total without any role color', () => {
      const wrapper = mount(Column, { props: baseProps });
      expect(spanWithText(wrapper, 'Je donne').attributes('class')).not.toMatch(ROLE_TEXT);
      expect(spanWithText(wrapper, formatPrice(5200)).attributes('class')).not.toMatch(ROLE_TEXT);
    });
  });

  it('shows a "Réservée" tag per card when reserved', () => {
    const wrapper = mount(Column, {
      props: { ...baseProps, removable: false, reserved: true },
    });
    const tags = wrapper.text().match(/Réservée/g) ?? [];
    expect(tags).toHaveLength(cards.length);
  });
});
