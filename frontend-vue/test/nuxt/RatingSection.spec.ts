import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import RatingSection from '~/components/Trade/RatingSection.vue';

const baseProps = { partner: 'mizzix_42', meRating: null, partnerRating: null, busy: false };

const skipButton = (wrapper: ReturnType<typeof mount>) =>
  wrapper.findAll('button').find((b) => b.text() === 'Passer la notation');

describe('RatingSection', () => {
  it('offers the stars and emits the chosen rating', async () => {
    const wrapper = mount(RatingSection, { props: baseProps });
    expect(wrapper.text()).toContain('Noter @mizzix_42');
    await wrapper.findAll('button[aria-label^="Noter "]')[3]!.trigger('click');
    expect(wrapper.emitted('rate')).toEqual([[4]]);
  });

  it('sends a 0 to skip the rating', async () => {
    const wrapper = mount(RatingSection, { props: baseProps });
    await skipButton(wrapper)!.trigger('click');
    expect(wrapper.emitted('rate')).toEqual([[0]]);
  });

  it('shows my rating once given', () => {
    const wrapper = mount(RatingSection, { props: { ...baseProps, meRating: 4 } });
    expect(wrapper.get('[data-part="mine"]').text()).toBe('Tu as mis 4/5');
    expect(skipButton(wrapper)).toBeUndefined();
  });

  it('reads my 0 as a skipped rating', () => {
    const wrapper = mount(RatingSection, { props: { ...baseProps, meRating: 0 } });
    expect(wrapper.get('[data-part="mine"]').text()).toBe('Notation passée');
  });

  it.each([
    [null, "Optionnel. L'échange se clôture dès que vous avez tous les deux noté ou passé."],
    [3, "@mizzix_42 t'a mis 3/5."],
    [0, '@mizzix_42 a passé la notation.'],
  ])('captions the partner rating %s', (partnerRating, caption) => {
    const wrapper = mount(RatingSection, { props: { ...baseProps, partnerRating } });
    expect(wrapper.get('[data-part="caption"]').text()).toBe(caption);
  });
});
