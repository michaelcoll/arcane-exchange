import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import StatusBanner from '~/components/Trade/StatusBanner.vue';
import type { TradeViewBanner } from '~/composables/useTradeView';

const banner = (overrides: Partial<TradeViewBanner> = {}): TradeViewBanner => ({
  tone: 'primary',
  icon: 'lucide:lock',
  title: 'En attente de Gandalf',
  text: 'Gandalf doit accepter à son tour.',
  ...overrides,
});

describe('StatusBanner', () => {
  it('shows the title and the text of the banner it is given', () => {
    const wrapper = mount(StatusBanner, { props: { banner: banner() } });

    expect(wrapper.text()).toContain('En attente de Gandalf');
    expect(wrapper.text()).toContain('Gandalf doit accepter à son tour.');
  });

  it.each(['primary', 'secondary', 'good', 'down', 'muted'] as const)(
    'carries the %s tone of the banner',
    (tone) => {
      const wrapper = mount(StatusBanner, { props: { banner: banner({ tone }) } });

      expect(wrapper.attributes('data-tone')).toBe(tone);
    },
  );
});
