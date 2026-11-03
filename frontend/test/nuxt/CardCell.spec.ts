import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import CardCell from '~/components/Card/Cell.vue';

const base = { name: 'Lightning Bolt', clickable: false };

// Intl uses a narrow no-break space before the euro sign and as thousands separator.
const text = (wrapper: ReturnType<typeof mount>) => wrapper.text().replace(/\s/g, ' ');

describe('CardCell', () => {
  it('shows the plain price without a comparison when there is no purchase price', () => {
    const wrapper = mount(CardCell, { props: { ...base, price: 250 } });

    expect(text(wrapper)).toContain('2,50 €');
    expect(wrapper.find('.line-through').exists()).toBe(false);
    expect(text(wrapper)).not.toContain('%');
  });

  it('shows the trend, the crossed-out purchase price and a signed badge on a significant gap', () => {
    const wrapper = mount(CardCell, {
      props: { ...base, purchased: 100, trend: 150 },
    });

    expect(text(wrapper)).toContain('1,50 €');
    expect(wrapper.find('.line-through').text().replace(/\s/g, ' ')).toBe('1 €');
    expect(text(wrapper)).toContain('+50%');
  });

  it('shows the trend alone on a neutral gap', () => {
    const wrapper = mount(CardCell, {
      props: { ...base, purchased: 100, trend: 101 },
    });

    expect(text(wrapper)).toContain('1,01 €');
    expect(wrapper.find('.line-through').exists()).toBe(false);
    expect(text(wrapper)).not.toContain('%');
  });

  it('shows the purchase price alone when it is zero', () => {
    const wrapper = mount(CardCell, {
      props: { ...base, purchased: 0, trend: 150 },
    });

    expect(text(wrapper)).toContain('0 €');
    expect(text(wrapper)).not.toContain('1,50 €');
    expect(wrapper.find('.line-through').exists()).toBe(false);
  });
});
