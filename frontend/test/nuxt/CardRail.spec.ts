import { describe, it, expect, beforeAll } from 'vitest';
import { config, mount } from '@vue/test-utils';
import CardRail from '~/components/Trade/CardRail.vue';
import type { TradeCard } from '~/bindings/TradeCard';
import type { TradeViewSide } from '~/composables/useTradeView';
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

// Le total vient de la vue : volontairement différent de la somme des cartes (5200).
const side: TradeViewSide = { cards, total: 9900 };
const emptySide: TradeViewSide = { cards: [], total: 0 };

const commonProps = {
  owner: 'mizzix_42',
  direction: 'receive' as const,
  side,
  reserved: false,
  removable: false,
  emptyMessage: "Tu n'as demandé aucune carte pour l'instant.",
};

// PlayerAvatar loads the avatar through Clerk: out of scope here.
beforeAll(() => {
  config.global.stubs = { ...config.global.stubs, PlayerAvatar: true };
});

describe('CardRail layout', () => {
  it('scrolls the tiles in a rail by default', () => {
    const wrapper = mount(CardRail, { props: commonProps });
    expect(wrapper.get('[data-part="tiles"]').attributes('data-layout')).toBe('rail');
  });

  it('wraps the tiles in a grid when asked to', () => {
    const wrapper = mount(CardRail, { props: { ...commonProps, layout: 'grid' } });
    expect(wrapper.get('[data-part="tiles"]').attributes('data-layout')).toBe('grid');
  });
});

// Only the tiles' layout changes: every behaviour holds for both variants.
describe.each(['rail', 'grid'] as const)('CardRail (%s)', (layout) => {
  const baseProps = { ...commonProps, layout };

  it('names who puts the cards down and shows the total of the side it received', () => {
    const wrapper = mount(CardRail, { props: baseProps });
    const header = wrapper.get('[data-part="header"]');
    expect(header.text()).toContain('@MIZZIX_42');
    expect(header.text()).toContain('POSE');
    expect(header.text()).toContain(formatPrice(9900));
  });

  it('reads « JE POSE » on my own side', () => {
    const wrapper = mount(CardRail, { props: { ...baseProps, owner: null, direction: 'give' } });
    expect(wrapper.get('[data-part="header"]').text()).toContain('JE POSE');
  });

  it('lays out one tile per card with its name and value', () => {
    const wrapper = mount(CardRail, { props: baseProps });
    const tiles = wrapper.findAll('[data-part="tile"]');
    expect(tiles).toHaveLength(2);
    expect(tiles[1]!.text()).toContain('Vampiric Tutor');
    expect(tiles[1]!.text()).toContain(formatPrice(4000));
  });

  it('emits "open" with the card when its face is clicked', async () => {
    const wrapper = mount(CardRail, { props: baseProps });
    await wrapper
      .findAll('[data-part="face"]')[1]!
      .get('[title="Vampiric Tutor"]')
      .trigger('click');
    expect(wrapper.emitted('open')).toEqual([[cards[1]]]);
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

  it('disables the remove badges while the view is busy', () => {
    const idle = mount(CardRail, { props: { ...baseProps, removable: true } });
    const busy = mount(CardRail, { props: { ...baseProps, removable: true, busy: true } });
    const disabled = (wrapper: ReturnType<typeof mount>) =>
      wrapper
        .findAll('button[aria-label^="Retirer "]')
        .map((b) => b.attributes('disabled') !== undefined);
    expect(disabled(idle)).toEqual([false, false]);
    expect(disabled(busy)).toEqual([true, true]);
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
    const wrapper = mount(CardRail, { props: { ...baseProps, side: emptySide } });
    const ghost = wrapper.get('[data-part="ghost"]');
    expect(ghost.text()).toBe('');
    expect(ghost.attributes('aria-label')).toBe(baseProps.emptyMessage);
  });

  it('leaves the add slot alone to explain an empty side that can grow', () => {
    const wrapper = mount(CardRail, {
      props: { ...baseProps, side: emptySide, addLabel: 'Chercher chez lui' },
    });
    expect(wrapper.find('[data-part="ghost"]').exists()).toBe(false);
    expect(wrapper.find('[data-part="add"]').exists()).toBe(true);
  });

  it('shows values in secondary on the partner side only', () => {
    const receive = mount(CardRail, { props: baseProps }).get('[data-part="total"]');
    const give = mount(CardRail, {
      props: { ...baseProps, owner: null, direction: 'give' },
    }).get('[data-part="total"]');
    expect(receive.classes()).toContain('text-secondary');
    expect(give.classes()).not.toContain('text-secondary');
  });
});
