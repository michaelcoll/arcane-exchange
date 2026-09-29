import { describe, it, expect, beforeAll } from 'vitest';
import { config, mount } from '@vue/test-utils';
import CardRail from '~/components/Trade/CardRail.vue';
import type { TradeCard } from '~/bindings/TradeCard';
import formatPrice from '~/utils/format-price';

const card = (overrides: Partial<TradeCard> = {}): TradeCard => ({
  set_code: 'ECL',
  collector_number: '166',
  language_code: 'FR',
  foil: false,
  name: 'Sol Ring',
  quantity: 1,
  price_guide: { low: 0, avg: 0, trend: 1200 },
  scryfall_id: 'sol-ring',
  image_url: null,
  image_back_url: null,
  ...overrides,
});

const cards: TradeCard[] = [
  card({ name: 'Sol Ring', collector_number: '166' }),
  card({
    name: 'Vampiric Tutor',
    collector_number: '167',
    quantity: 2,
    price_guide: { low: 0, avg: 0, trend: 2000 },
  }),
];

const baseProps = {
  owner: 'mizzix_42',
  side: 'receive' as const,
  cards,
  reserved: false,
  removable: false,
  emptyMessage: "Tu n'as demandé aucune carte pour l'instant.",
};

describe('CardRail', () => {
  // PlayerAvatar charge l'avatar via Clerk : hors sujet ici.
  beforeAll(() => {
    config.global.stubs = { ...config.global.stubs, PlayerAvatar: true };
  });

  it('names who puts the cards down and totals the side in the header', () => {
    const wrapper = mount(CardRail, { props: baseProps });
    const header = wrapper.get('[data-part="header"]');
    expect(header.text()).toContain('@MIZZIX_42');
    expect(header.text()).toContain('POSE');
    // 1200 + 2000×2
    expect(header.text()).toContain(formatPrice(5200));
  });

  it('reads « JE POSE » on my own side', () => {
    const wrapper = mount(CardRail, { props: { ...baseProps, owner: null, side: 'give' } });
    expect(wrapper.get('[data-part="header"]').text()).toContain('JE POSE');
  });

  it('lays out one tile per card with its name and value', () => {
    const wrapper = mount(CardRail, { props: baseProps });
    const tiles = wrapper.findAll('[data-part="tile"]');
    expect(tiles).toHaveLength(2);
    expect(tiles[1]!.text()).toContain('Vampiric Tutor');
    expect(tiles[1]!.text()).toContain(formatPrice(4000));
  });

  it('shows ×N only when the quantity is greater than 1', () => {
    const tiles = mount(CardRail, { props: baseProps }).findAll('[data-part="tile"]');
    expect(tiles[0]!.text()).not.toContain('×');
    expect(tiles[1]!.text()).toContain('×2');
  });

  it('puts a remove badge on every tile when removable, and emits the card', async () => {
    const wrapper = mount(CardRail, { props: { ...baseProps, removable: true } });
    const remove = wrapper.findAll('button[aria-label^="Retirer "]');
    expect(remove).toHaveLength(2);
    expect(remove[1]!.attributes('aria-label')).toBe('Retirer Vampiric Tutor');
    await remove[1]!.trigger('click');
    expect(wrapper.emitted('remove')).toEqual([[cards[1]]]);
  });

  it('has no remove badge when not removable', () => {
    const wrapper = mount(CardRail, { props: baseProps });
    expect(wrapper.find('button[aria-label^="Retirer "]').exists()).toBe(false);
  });

  it('locks and dims reserved cards that cannot be removed', () => {
    const wrapper = mount(CardRail, { props: { ...baseProps, reserved: true } });
    expect(wrapper.findAll('[aria-label="Carte réservée"]')).toHaveLength(2);
    for (const face of wrapper.findAll('[data-part="face"]')) {
      expect(face.classes()).toContain('opacity-[0.82]');
    }
  });

  it('keeps the remove badge rather than the lock when a reserved card is still removable', () => {
    const wrapper = mount(CardRail, { props: { ...baseProps, reserved: true, removable: true } });
    expect(wrapper.find('[aria-label="Carte réservée"]').exists()).toBe(false);
    expect(wrapper.findAll('button[aria-label^="Retirer "]')).toHaveLength(2);
  });

  it('ends with the add slot when one is given, and emits "add"', async () => {
    const wrapper = mount(CardRail, { props: { ...baseProps, addLabel: 'Chercher chez lui' } });
    const slot = wrapper.get('[data-part="add"]');
    expect(slot.text()).toContain('Chercher chez lui');
    await slot.trigger('click');
    expect(wrapper.emitted('add')).toHaveLength(1);
  });

  it('shows a wordless ghost tile when the side is empty and cannot grow', () => {
    const wrapper = mount(CardRail, { props: { ...baseProps, cards: [] } });
    const ghost = wrapper.get('[data-part="ghost"]');
    expect(ghost.text()).toBe('');
    expect(ghost.attributes('aria-label')).toBe(baseProps.emptyMessage);
  });

  it('leaves the add slot alone to explain an empty side that can grow', () => {
    const wrapper = mount(CardRail, {
      props: { ...baseProps, cards: [], addLabel: 'Chercher chez lui' },
    });
    expect(wrapper.find('[data-part="ghost"]').exists()).toBe(false);
    expect(wrapper.find('[data-part="add"]').exists()).toBe(true);
  });

  it('shows values in secondary on the partner side only', () => {
    const receive = mount(CardRail, { props: baseProps }).get('[data-part="total"]');
    const give = mount(CardRail, {
      props: { ...baseProps, owner: null, side: 'give' },
    }).get('[data-part="total"]');
    expect(receive.classes()).toContain('text-secondary');
    expect(give.classes()).not.toContain('text-secondary');
  });
});
