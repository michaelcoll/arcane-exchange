import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import StatusBanner from '~/components/Trade/StatusBanner.vue';

const baseProps = {
  counterparty: 'Gandalf',
  accepted: false,
  confirmed: false,
  abandonedByMe: false,
};

describe('StatusBanner', () => {
  it('shows the waiting message when the current user accepted first', async () => {
    const wrapper = mount(StatusBanner, {
      props: { ...baseProps, status: 'ONE_ACCEPTED', accepted: true },
    });
    expect(wrapper.text()).toContain('En attente de Gandalf');
  });

  it('shows the "your turn" message when the counterparty accepted first', async () => {
    const wrapper = mount(StatusBanner, {
      props: { ...baseProps, status: 'ONE_ACCEPTED', accepted: false },
    });
    expect(wrapper.text()).toContain('Gandalf a accepté');
  });

  it('shows the confirmation-pending message once locked and confirmed by me', async () => {
    const wrapper = mount(StatusBanner, {
      props: { ...baseProps, status: 'FULLY_ACCEPTED', confirmed: true },
    });
    expect(wrapper.text()).toContain("Tu as confirmé l'échange");
  });

  it('prompts for the physical exchange once locked and not yet confirmed', async () => {
    const wrapper = mount(StatusBanner, {
      props: { ...baseProps, status: 'FULLY_ACCEPTED', confirmed: false },
    });
    expect(wrapper.text()).toContain('Procédez à l’échange physique');
  });

  it('attributes the abandon to the current user', async () => {
    const wrapper = mount(StatusBanner, {
      props: { ...baseProps, status: 'ABANDONED', abandonedByMe: true },
    });
    expect(wrapper.text()).toContain('Tu as abandonné cet échange');
  });

  it('attributes the abandon to the counterparty', async () => {
    const wrapper = mount(StatusBanner, {
      props: { ...baseProps, status: 'ABANDONED', abandonedByMe: false },
    });
    expect(wrapper.text()).toContain('Gandalf a abandonné');
  });

  it('falls back to the open-negotiation message for PENDING', async () => {
    const wrapper = mount(StatusBanner, { props: { ...baseProps, status: 'PENDING' } });
    expect(wrapper.text()).toContain('Négociation ouverte');
    expect(wrapper.text()).toContain('notifie Gandalf');
  });

  it('shows the completed message', async () => {
    const wrapper = mount(StatusBanner, { props: { ...baseProps, status: 'COMPLETED' } });
    expect(wrapper.text()).toContain('Échange réalisé');
  });

  it('shows the closed message', async () => {
    const wrapper = mount(StatusBanner, { props: { ...baseProps, status: 'CLOSED' } });
    expect(wrapper.text()).toContain('Transaction clôturée');
  });
});
