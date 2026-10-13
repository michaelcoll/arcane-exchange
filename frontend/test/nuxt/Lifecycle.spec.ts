import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import Lifecycle from '~/components/Trade/Lifecycle.vue';
import { STEP_TITLES, stepsAt } from './tradeSteps';

describe('Lifecycle', () => {
  it('checks the steps the view reports as done and numbers the others', () => {
    const wrapper = mount(Lifecycle, { props: { steps: stepsAt(2) } });
    expect(wrapper.findAll('.iconify')).toHaveLength(2);
    expect(wrapper.text()).toContain('3');
    expect(wrapper.text()).toContain('5');
  });

  it('renders the name of every step it receives', () => {
    const wrapper = mount(Lifecycle, { props: { steps: stepsAt(0) } });
    for (const title of STEP_TITLES) {
      expect(wrapper.text()).toContain(title);
    }
  });
});
