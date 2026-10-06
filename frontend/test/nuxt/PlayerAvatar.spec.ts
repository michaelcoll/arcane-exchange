import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import PlayerAvatar from '~/components/PlayerAvatar.vue';
import type { UserProfileResponse } from '~/bindings/UserProfileResponse';

const { getUserProfileMock } = vi.hoisted(() => ({
  getUserProfileMock: vi.fn(),
}));

vi.mock('~/composables/useUserService', () => ({
  useUserService: () => ({ getUserProfile: getUserProfileMock }),
}));

describe('PlayerAvatar', () => {
  beforeEach(() => {
    getUserProfileMock.mockReset();
  });

  it('affiche l’image quand avatar_url est présent', async () => {
    getUserProfileMock.mockResolvedValue({
      id: 'user_alice',
      username: 'alice',
      avatar_url: 'https://img.example.com/alice.png',
    });

    const wrapper = mount(PlayerAvatar, { props: { username: 'alice' } });
    await flushPromises();

    const img = wrapper.find('img');
    expect(img.exists()).toBe(true);
    expect(img.attributes('src')).toBe('https://img.example.com/alice.png');
  });

  it('affiche les initiales quand avatar_url est null', async () => {
    getUserProfileMock.mockResolvedValue({
      id: 'user_bob',
      username: 'bob',
      avatar_url: null,
    });

    const wrapper = mount(PlayerAvatar, { props: { username: 'bob' } });
    await flushPromises();

    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.text()).toBe('BO');
  });

  it('retombe sur les initiales en cas d’échec réseau et ne rejette pas', async () => {
    getUserProfileMock.mockRejectedValue(new Error('network down'));

    const wrapper = mount(PlayerAvatar, { props: { username: 'carol' } });
    await flushPromises();

    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.text()).toBe('CA');
  });

  it('retombe sur les initiales quand le chargement de l’image échoue', async () => {
    getUserProfileMock.mockResolvedValue({
      id: 'user_dave',
      username: 'dave',
      avatar_url: 'https://img.example.com/dave.png',
    });

    const wrapper = mount(PlayerAvatar, { props: { username: 'dave' } });
    await flushPromises();

    const img = wrapper.find('img');
    expect(img.exists()).toBe(true);

    await img.trigger('error');
    await flushPromises();

    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.text()).toBe('DA');
  });

  it('change de joueur sans jamais afficher l’avatar du précédent', async () => {
    let resolveEve!: (value: UserProfileResponse) => void;
    const evePending = new Promise<UserProfileResponse>((resolve) => {
      resolveEve = resolve;
    });
    getUserProfileMock
      .mockResolvedValueOnce({
        id: 'user_dora',
        username: 'dora',
        avatar_url: 'https://img.example.com/dora.png',
      })
      .mockImplementationOnce(() => evePending);

    const wrapper = mount(PlayerAvatar, { props: { username: 'dora' } });
    await flushPromises();
    expect(wrapper.find('img').attributes('src')).toBe('https://img.example.com/dora.png');

    await wrapper.setProps({ username: 'eve' });

    // Pendant la requête du nouveau joueur : initiales, jamais l’ancienne image.
    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.text()).toBe('EV');

    resolveEve({
      id: 'user_eve',
      username: 'eve',
      avatar_url: 'https://img.example.com/eve.png',
    });
    await flushPromises();

    const img = wrapper.find('img');
    expect(img.exists()).toBe(true);
    expect(img.attributes('src')).toBe('https://img.example.com/eve.png');
  });
});
