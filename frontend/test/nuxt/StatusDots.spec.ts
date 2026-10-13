import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import StatusDots from '~/components/Trade/StatusDots.vue';
import type { TradeStatusPresentation } from '~/utils/trade';
import { stepsAt } from './tradeSteps';

const dots = (wrapper: ReturnType<typeof mount>, state: 'done' | 'upcoming') =>
  wrapper.findAll(`[data-dot="${state}"]`);

const presentation = (
  overrides: Partial<TradeStatusPresentation> = {},
): TradeStatusPresentation => ({ label: 'En négociation', tone: 'primary', ...overrides });

describe('StatusDots', () => {
  it('lights the steps already behind the trade and labels the current one', () => {
    const wrapper = mount(StatusDots, {
      props: { presentation: presentation(), steps: stepsAt(2) },
    });
    expect(dots(wrapper, 'done')).toHaveLength(2);
    expect(dots(wrapper, 'upcoming')).toHaveLength(2);
    expect(wrapper.get('[data-part="pill"]').text()).toBe('VERROUILLÉ · 3/5');
  });

  it('places the pill in its own position among the dots', () => {
    const wrapper = mount(StatusDots, {
      props: { presentation: presentation(), steps: stepsAt(1) },
    });
    const parts = wrapper
      .findAll('[data-dot], [data-part="pill"]')
      .map((el) => el.attributes('data-dot') ?? 'pill');
    expect(parts).toEqual(['done', 'pill', 'upcoming', 'upcoming', 'upcoming']);
  });

  it('turns every dot off and trails the pill without a step number once abandoned', () => {
    const wrapper = mount(StatusDots, {
      props: { presentation: presentation(), steps: stepsAt(null) },
    });
    expect(dots(wrapper, 'done')).toHaveLength(0);
    expect(dots(wrapper, 'upcoming')).toHaveLength(5);
    const parts = wrapper
      .findAll('[data-dot], [data-part="pill"]')
      .map((el) => el.attributes('data-dot') ?? 'pill');
    expect(parts.at(-1)).toBe('pill');
    expect(wrapper.get('[data-part="pill"]').text()).toBe('ABANDONNÉ');
  });

  it.each(['primary', 'secondary', 'good', 'down', 'muted'] as const)(
    'colours the pill with the %s tone of the status',
    (tone) => {
      const wrapper = mount(StatusDots, {
        props: { presentation: presentation({ tone }), steps: stepsAt(0) },
      });
      expect(wrapper.get('[data-part="pill"]').attributes('data-tone')).toBe(tone);
    },
  );

  it('announces the status and the step to assistive technologies', () => {
    const wrapper = mount(StatusDots, {
      props: { presentation: presentation({ label: 'Verrouillé' }), steps: stepsAt(2) },
    });
    expect(wrapper.get('[role="img"]').attributes('aria-label')).toBe('Verrouillé, étape 3 sur 5');
  });

  it('announces the status alone when no step is current', () => {
    const wrapper = mount(StatusDots, {
      props: { presentation: presentation({ label: 'Abandonnée' }), steps: stepsAt(null) },
    });
    expect(wrapper.get('[role="img"]').attributes('aria-label')).toBe('Abandonnée');
  });

  it('emits "help" from the ⓘ button', async () => {
    const wrapper = mount(StatusDots, {
      props: { presentation: presentation(), steps: stepsAt(0) },
    });
    await wrapper.get('button[aria-label="Les étapes d\'un échange"]').trigger('click');
    expect(wrapper.emitted('help')).toHaveLength(1);
  });
});
