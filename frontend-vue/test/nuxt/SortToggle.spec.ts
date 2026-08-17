import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import SortToggle from '~/components/SortToggle.vue';

const options = [
  { value: 'trend', label: 'Prix', ascLabel: 'Prix croissant', descLabel: 'Prix décroissant' },
  {
    value: 'added_at',
    label: 'Date d’ajout',
    ascLabel: 'Plus ancien d’abord',
    descLabel: 'Plus récent d’abord',
  },
];

describe('SortToggle', () => {
  it('renders one button per option', () => {
    const wrapper = mount(SortToggle, {
      props: { modelValue: { sort_by: 'trend', sort_dir: 'desc' }, options },
    });
    expect(wrapper.findAll('button')).toHaveLength(2);
  });

  it('shows the descLabel on the active option when sort_dir is desc', () => {
    const wrapper = mount(SortToggle, {
      props: { modelValue: { sort_by: 'trend', sort_dir: 'desc' }, options },
    });
    expect(wrapper.text()).toContain('Prix décroissant');
  });

  it('shows the ascLabel on the active option when sort_dir is asc', () => {
    const wrapper = mount(SortToggle, {
      props: { modelValue: { sort_by: 'trend', sort_dir: 'asc' }, options },
    });
    expect(wrapper.text()).toContain('Prix croissant');
  });

  it('shows the plain label on inactive options, never their asc/desc variants', () => {
    const wrapper = mount(SortToggle, {
      props: { modelValue: { sort_by: 'trend', sort_dir: 'desc' }, options },
    });
    expect(wrapper.text()).toContain('Date d’ajout');
    expect(wrapper.text()).not.toContain('Plus récent');
    expect(wrapper.text()).not.toContain('Plus ancien');
  });

  it('clicking the active option toggles sort_dir and keeps sort_by', async () => {
    const wrapper = mount(SortToggle, {
      props: { modelValue: { sort_by: 'trend', sort_dir: 'desc' }, options },
    });
    await wrapper.findAll('button')[0]!.trigger('click');
    expect(wrapper.emitted('update:modelValue')).toEqual([[{ sort_by: 'trend', sort_dir: 'asc' }]]);
  });

  it('clicking an inactive option switches sort_by and keeps the current sort_dir', async () => {
    const wrapper = mount(SortToggle, {
      props: { modelValue: { sort_by: 'trend', sort_dir: 'asc' }, options },
    });
    await wrapper.findAll('button')[1]!.trigger('click');
    expect(wrapper.emitted('update:modelValue')).toEqual([
      [{ sort_by: 'added_at', sort_dir: 'asc' }],
    ]);
  });
});
