import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import RatingSection from '~/components/Trade/RatingSection.vue';
import type { TradeViewRating } from '~/composables/useTradeView';

const open: TradeViewRating = { visible: true, canRate: true, mine: null, partner: null };

const baseProps = { partner: 'mizzix_42', rating: open, busy: false };

const skipButton = (wrapper: ReturnType<typeof mount>) =>
  wrapper.findAll('button').find((b) => b.text() === 'Passer la notation');

describe('RatingSection', () => {
  it('offers the stars while I can rate, and emits the chosen rating', async () => {
    const wrapper = mount(RatingSection, { props: baseProps });
    expect(wrapper.text()).toContain('Noter @mizzix_42');
    await wrapper.findAll('button[aria-label^="Noter "]')[3]!.trigger('click');
    expect(wrapper.emitted('rate')).toEqual([[4]]);
  });

  it('emits "skip" to skip the rating', async () => {
    const wrapper = mount(RatingSection, { props: baseProps });
    await skipButton(wrapper)!.trigger('click');
    expect(wrapper.emitted('skip')).toHaveLength(1);
    expect(wrapper.emitted('rate')).toBeUndefined();
  });

  it('shows my rating once given', () => {
    const wrapper = mount(RatingSection, {
      props: { ...baseProps, rating: { ...open, canRate: false, mine: 4 } },
    });
    expect(wrapper.get('[data-part="mine"]').text()).toBe('Tu as mis 4/5');
    expect(skipButton(wrapper)).toBeUndefined();
  });

  it('reads my 0 as a skipped rating', () => {
    const wrapper = mount(RatingSection, {
      props: { ...baseProps, rating: { ...open, canRate: false, mine: 0 } },
    });
    expect(wrapper.get('[data-part="mine"]').text()).toBe('Notation passée');
  });

  it('offers nothing to rate with when the view says I cannot rate', () => {
    const wrapper = mount(RatingSection, {
      props: { ...baseProps, rating: { ...open, canRate: false } },
    });
    expect(wrapper.find('button').exists()).toBe(false);
    expect(wrapper.find('[data-part="mine"]').exists()).toBe(false);
  });

  it('disables the skip button while busy', () => {
    const wrapper = mount(RatingSection, { props: { ...baseProps, busy: true } });
    expect(skipButton(wrapper)!.attributes('disabled')).toBeDefined();
  });

  it.each([
    [null, "Optionnel. L'échange se clôture dès que vous avez tous les deux noté ou passé."],
    [3, "@mizzix_42 t'a mis 3/5."],
    [0, '@mizzix_42 a passé la notation.'],
  ])('captions the partner rating %s', (partner, caption) => {
    const wrapper = mount(RatingSection, {
      props: { ...baseProps, rating: { ...open, partner } },
    });
    expect(wrapper.get('[data-part="caption"]').text()).toBe(caption);
  });
});
