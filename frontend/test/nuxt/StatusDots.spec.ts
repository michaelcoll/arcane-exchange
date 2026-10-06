import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import StatusDots from '~/components/Trade/StatusDots.vue';

const dots = (wrapper: ReturnType<typeof mount>, state: 'done' | 'todo') =>
  wrapper.findAll(`[data-dot="${state}"]`);

describe('StatusDots', () => {
  it('lights the steps already behind the trade and labels the current one', () => {
    const wrapper = mount(StatusDots, { props: { status: 'FULLY_ACCEPTED' } });
    expect(dots(wrapper, 'done')).toHaveLength(2);
    expect(dots(wrapper, 'todo')).toHaveLength(2);
    expect(wrapper.get('[data-part="pill"]').text()).toBe('VERROUILLÉ · 3/5');
  });

  it('places the pill in its own position among the dots', () => {
    const wrapper = mount(StatusDots, { props: { status: 'ONE_ACCEPTED' } });
    const parts = wrapper
      .findAll('[data-dot], [data-part="pill"]')
      .map((el) => el.attributes('data-dot') ?? 'pill');
    expect(parts).toEqual(['done', 'pill', 'todo', 'todo', 'todo']);
  });

  it('turns every dot off and trails the pill without a step number once abandoned', () => {
    const wrapper = mount(StatusDots, { props: { status: 'ABANDONED' } });
    expect(dots(wrapper, 'done')).toHaveLength(0);
    expect(dots(wrapper, 'todo')).toHaveLength(5);
    const parts = wrapper
      .findAll('[data-dot], [data-part="pill"]')
      .map((el) => el.attributes('data-dot') ?? 'pill');
    expect(parts.at(-1)).toBe('pill');
    expect(wrapper.get('[data-part="pill"]').text()).toBe('ABANDONNÉ');
  });

  it('announces the step to assistive technologies', () => {
    const wrapper = mount(StatusDots, { props: { status: 'PENDING' } });
    expect(wrapper.get('[role="img"]').attributes('aria-label')).toBe(
      'En négociation, étape 1 sur 5',
    );
  });

  it('emits "help" from the ⓘ button', async () => {
    const wrapper = mount(StatusDots, { props: { status: 'PENDING' } });
    await wrapper.get('button[aria-label="Les étapes d\'un échange"]').trigger('click');
    expect(wrapper.emitted('help')).toHaveLength(1);
  });
});
