import { describe, it, expect, afterEach } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import StepsSheet from '~/components/Trade/StepsSheet.vue';

const states = (wrapper: VueWrapper) =>
  wrapper.findAll('li').map((li) => li.attributes('data-state'));

describe('StepsSheet', () => {
  let wrapper: VueWrapper | undefined;

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
  });

  it('calls out the current step', () => {
    wrapper = mount(StepsSheet, { props: { status: 'FULLY_ACCEPTED', partner: 'mizzix_42' } });
    expect(states(wrapper)).toEqual(['done', 'done', 'current', 'upcoming', 'upcoming']);
    expect(wrapper.findAll('[data-part="current"]')).toHaveLength(1);
    expect(wrapper.text()).toContain("Tu es à l'étape 3.");
  });

  it('marks no step as current once abandoned', () => {
    wrapper = mount(StepsSheet, { props: { status: 'ABANDONED', partner: 'mizzix_42' } });
    expect(states(wrapper)).toEqual(Array(5).fill('upcoming'));
    expect(wrapper.text()).toContain('Les cartes réservées ont été libérées.');
  });

  it('closes on Escape and from the close button', async () => {
    wrapper = mount(StepsSheet, { props: { status: 'PENDING', partner: 'mizzix_42' } });
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await wrapper.get('button[aria-label="Fermer"]').trigger('click');
    expect(wrapper.emitted('close')).toHaveLength(2);
  });
});
