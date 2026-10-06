import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import RatingStars from '~/components/Trade/RatingStars.vue';

describe('RatingStars', () => {
  it('renders 5 star buttons', async () => {
    const wrapper = mount(RatingStars, { props: { value: null } });
    expect(wrapper.findAll('button')).toHaveLength(5);
  });

  it('emits "rate" with the clicked star index', async () => {
    const wrapper = mount(RatingStars, { props: { value: null } });
    await wrapper.findAll('button')[2]!.trigger('click');
    expect(wrapper.emitted('rate')).toEqual([[3]]);
  });

  it('does not emit "rate" when read-only', async () => {
    const wrapper = mount(RatingStars, { props: { value: 3, readOnly: true } });
    await wrapper.findAll('button')[4]!.trigger('click');
    expect(wrapper.emitted('rate')).toBeUndefined();
  });

  it('disables the buttons when read-only', async () => {
    const wrapper = mount(RatingStars, { props: { value: 3, readOnly: true } });
    for (const button of wrapper.findAll('button')) {
      expect(button.attributes('disabled')).toBeDefined();
    }
  });

  it('does not disable the buttons by default', async () => {
    const wrapper = mount(RatingStars, { props: { value: null } });
    for (const button of wrapper.findAll('button')) {
      expect(button.attributes('disabled')).toBeUndefined();
    }
  });
});
