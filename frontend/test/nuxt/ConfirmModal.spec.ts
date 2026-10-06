import { describe, it, expect, afterEach } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import ConfirmModal from '~/components/Trade/ConfirmModal.vue';

const baseProps = { title: 'Titre', body: 'Corps', confirmLabel: 'Valider' };

describe('ConfirmModal', () => {
  let wrapper: VueWrapper | undefined;

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
  });

  it('renders the title, body and confirm label', async () => {
    wrapper = mount(ConfirmModal, { props: baseProps });
    expect(wrapper.text()).toContain('Titre');
    expect(wrapper.text()).toContain('Corps');
    expect(wrapper.text()).toContain('Valider');
  });

  it('emits "confirm" when the confirm button is clicked', async () => {
    wrapper = mount(ConfirmModal, { props: baseProps });
    await wrapper.findAll('button')[1]!.trigger('click');
    expect(wrapper.emitted('confirm')).toHaveLength(1);
  });

  it('emits "cancel" when the cancel button is clicked', async () => {
    wrapper = mount(ConfirmModal, { props: baseProps });
    await wrapper.findAll('button')[0]!.trigger('click');
    expect(wrapper.emitted('cancel')).toHaveLength(1);
  });

  it('emits "cancel" when clicking the backdrop', async () => {
    wrapper = mount(ConfirmModal, { props: baseProps });
    await wrapper.find('[role="dialog"]').trigger('click');
    expect(wrapper.emitted('cancel')).toHaveLength(1);
  });

  it('does not emit "cancel" when clicking inside the dialog panel', async () => {
    wrapper = mount(ConfirmModal, { props: baseProps });
    await wrapper.find('h3').trigger('click');
    expect(wrapper.emitted('cancel')).toBeUndefined();
  });

  it('emits "cancel" on Escape key while mounted', async () => {
    wrapper = mount(ConfirmModal, { props: baseProps });
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(wrapper.emitted('cancel')).toHaveLength(1);
  });

  it('stops listening for Escape after unmount', async () => {
    wrapper = mount(ConfirmModal, { props: baseProps });
    wrapper.unmount();
    expect(() =>
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })),
    ).not.toThrow();
  });
});
