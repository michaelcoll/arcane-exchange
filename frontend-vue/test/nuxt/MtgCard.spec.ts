import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import MtgCard from '~/components/MtgCard.vue';

const FRONT = '/card-images/ISD_51_EN.webp?v=gatherer';
const BACK = '/card-images/ISD_51_EN_back.webp?v=gatherer';

const sources = (wrapper: ReturnType<typeof mount>) =>
  wrapper.findAll('img').map((img) => img.attributes('src'));

describe('MtgCard', () => {
  it('shows the stored image of the card', () => {
    const wrapper = mount(MtgCard, { props: { imageUrl: FRONT, name: 'Delver of Secrets' } });
    expect(sources(wrapper)).toEqual([FRONT]);
  });

  it('shows the generic card back while the image is pending', () => {
    const wrapper = mount(MtgCard, { props: { imageUrl: null, name: 'Delver of Secrets' } });
    expect(sources(wrapper)).toEqual(['/card-back.webp']);
  });

  it('shows the generic card back when the image fails to load', async () => {
    const wrapper = mount(MtgCard, { props: { imageUrl: FRONT } });
    await wrapper.find('img').trigger('error');
    expect(sources(wrapper)).toEqual(['/card-back.webp']);
  });

  it('only offers to flip a double-faced card that is flippable', () => {
    const notFlippable = mount(MtgCard, { props: { imageUrl: FRONT, imageBackUrl: BACK } });
    expect(notFlippable.find('[data-testid="flip"]').exists()).toBe(false);
    expect(sources(notFlippable)).toEqual([FRONT]);

    const singleFaced = mount(MtgCard, { props: { imageUrl: FRONT, flippable: true } });
    expect(singleFaced.find('[data-testid="flip"]').exists()).toBe(false);
  });

  it('flips a double-faced card to its back and back again', async () => {
    const wrapper = mount(MtgCard, {
      props: { imageUrl: FRONT, imageBackUrl: BACK, flippable: true },
    });
    expect(sources(wrapper)).toEqual([FRONT, BACK]);
    const flip = wrapper.find('[data-testid="flip"]');
    expect(flip.attributes('aria-pressed')).toBe('false');

    await flip.trigger('click');
    expect(flip.attributes('aria-pressed')).toBe('true');
    expect(wrapper.emitted('click')).toBeUndefined();

    await flip.trigger('click');
    expect(flip.attributes('aria-pressed')).toBe('false');
  });

  it('shows the front again when it becomes another card', async () => {
    const wrapper = mount(MtgCard, {
      props: { imageUrl: FRONT, imageBackUrl: BACK, flippable: true },
    });
    await wrapper.find('[data-testid="flip"]').trigger('click');
    await wrapper.setProps({ imageUrl: '/card-images/FDN_1_EN.webp?v=scryfall' });
    expect(wrapper.find('[data-testid="flip"]').attributes('aria-pressed')).toBe('false');
  });
});
