import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import CollectionFilters from '~/components/CollectionFilters.vue';
import type { RarityCode } from '~/bindings/RarityCode';
import { RARITY_COLOR_CLASS, RARITY_LABELS, RARITY_ORDER } from '~/utils/rarity';

const mountFilters = (rar: RarityCode[] = []) =>
  mount(CollectionFilters, {
    props: { active: { rar, sets: [] }, setList: [] },
  });

const rarityChip = (wrapper: ReturnType<typeof mountFilters>, r: RarityCode) =>
  wrapper.findAll('button').find((b) => b.text() === RARITY_LABELS[r])!;

describe('CollectionFilters rarity chips', () => {
  it.each(RARITY_ORDER)('shows a symbol tinted by rarity %s when inactive', (r) => {
    const chip = rarityChip(mountFilters(), r);
    expect(chip.find(`[data-rarity-symbol].${RARITY_COLOR_CLASS[r]}`).exists()).toBe(true);
  });

  it.each(RARITY_ORDER)('keeps the tinted symbol of rarity %s when active', (r) => {
    const chip = rarityChip(mountFilters([r]), r);
    expect(chip.find(`[data-rarity-symbol].${RARITY_COLOR_CLASS[r]}`).exists()).toBe(true);
    expect(chip.classes()).toContain('bg-primary/10');
  });

  it('emits a toggle with the rarity code on click', async () => {
    const wrapper = mountFilters();
    await rarityChip(wrapper, 'M').trigger('click');
    expect(wrapper.emitted('toggle')).toEqual([['rar', 'M']]);
  });
});
