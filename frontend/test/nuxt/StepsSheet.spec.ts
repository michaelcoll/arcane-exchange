import { describe, it, expect, afterEach } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import StepsSheet from '~/components/Trade/StepsSheet.vue';
import { STEP_TITLES, stepsAt } from './tradeSteps';

const states = (wrapper: VueWrapper) =>
  wrapper.findAll('li').map((li) => li.attributes('data-state'));

describe('StepsSheet', () => {
  let wrapper: VueWrapper | undefined;

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
  });

  it('calls out the current step', () => {
    wrapper = mount(StepsSheet, { props: { steps: stepsAt(2), partner: 'mizzix_42' } });
    expect(states(wrapper)).toEqual(['done', 'done', 'current', 'upcoming', 'upcoming']);
    expect(wrapper.findAll('[data-part="current"]')).toHaveLength(1);
    expect(wrapper.text()).toContain("Tu es à l'étape 3.");
  });

  it('explains every step it receives, name and detail', () => {
    wrapper = mount(StepsSheet, { props: { steps: stepsAt(0), partner: 'mizzix_42' } });
    for (const title of STEP_TITLES) {
      expect(wrapper.text()).toContain(title);
    }
    expect(wrapper.text()).toContain("Détail de l'étape 5");
  });

  it('marks no step as current once abandoned', () => {
    wrapper = mount(StepsSheet, { props: { steps: stepsAt(null), partner: 'mizzix_42' } });
    expect(states(wrapper)).toEqual(Array(5).fill('upcoming'));
    expect(wrapper.text()).toContain('Les cartes réservées ont été libérées.');
  });

  it('closes on Escape and from the close button', async () => {
    wrapper = mount(StepsSheet, { props: { steps: stepsAt(0), partner: 'mizzix_42' } });
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await wrapper.get('button[aria-label="Fermer"]').trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(2);
  });

  // On desktop the dialog sits inside the backdrop, centered: a click inside it must not bubble
  // up to the backdrop, and a click on the backdrop still closes the sheet.
  it('closes from the backdrop, not from a click inside the dialog', async () => {
    wrapper = mount(StepsSheet, { props: { steps: stepsAt(0), partner: 'mizzix_42' } });
    await wrapper.get('[role="dialog"]').trigger('click');
    expect(wrapper.emitted('close')).toBeUndefined();
    await wrapper.trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(1);
  });
});
