import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import CollectionFilters from '~/components/CollectionFilters.vue';
import type { RarityCode } from '~/bindings/RarityCode';
import { RARITY_LABELS, RARITY_ORDER } from '~/utils/rarity';

const mountFilters = (rar: RarityCode[] = []) =>
  mount(CollectionFilters, {
    props: { active: { rar, sets: [] }, setList: [] },
  });

const rarityChip = (wrapper: ReturnType<typeof mountFilters>, r: RarityCode) =>
  wrapper.findAll('button').find((b) => b.text() === RARITY_LABELS[r])!;

describe('CollectionFilters rarity chips', () => {
  it.each(RARITY_ORDER)('shows no rarity color on the inactive %s chip', (r) => {
    const chip = rarityChip(mountFilters(), r);
    expect(chip.classes().some((c) => c.includes('rarity-'))).toBe(false);
    expect(chip.find('svg, .iconify').exists()).toBe(false);
  });

  it.each(RARITY_ORDER)('keeps the usual chip style when %s is active', (r) => {
    const chip = rarityChip(mountFilters([r]), r);
    expect(chip.classes()).toContain('border-primary/30');
    expect(chip.classes().some((c) => c.includes('rarity-'))).toBe(false);
  });

  it('emits a toggle with the rarity code on click', async () => {
    const wrapper = mountFilters();
    await rarityChip(wrapper, 'M').trigger('click');
    expect(wrapper.emitted('toggle')).toEqual([['rar', 'M']]);
  });
});

describe('CollectionFilters active sets', () => {
  // On reload, `/search` reads the active sets from the URL before (and regardless of) the set
  // facet: a set the facet does not list, or not yet, must stay shown as applied and removable.
  it('keeps a set restored from the URL applied while the facet does not list it', async () => {
    const wrapper = mount(CollectionFilters, {
      props: {
        active: { rar: [], sets: ['MH3'] },
        setList: [{ code: 'LTR', name: 'The Lord of the Rings' }],
      },
    });

    expect(wrapper.text()).not.toContain('Toutes les extensions');
    const remove = wrapper.find('[aria-label="Retirer MH3"]');
    expect(remove.exists()).toBe(true);

    await remove.trigger('click');
    expect(wrapper.emitted('toggle')).toEqual([['sets', 'MH3']]);
  });
});
