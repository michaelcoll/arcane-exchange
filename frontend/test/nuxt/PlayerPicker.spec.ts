import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mockNuxtImport, mountSuspended, registerEndpoint } from '@nuxt/test-utils/runtime';
import type { Ref } from 'vue';
import type { UserSuggestion } from '~/bindings/UserSuggestion';
import PlayerPicker from '~/components/PlayerPicker.vue';

// `useAuth` is also called by app.vue and the register-user plugin while the Nuxt test app
// boots, so the mocked state must hold refs from the start.
const { authState, getTokenMock } = await vi.hoisted(async () => {
  const { ref } = await import('vue');
  const getTokenMock = vi.fn();
  return {
    getTokenMock,
    authState: {
      isLoaded: ref(true) as Ref<boolean>,
      isSignedIn: ref<boolean | undefined>(false),
      getToken: ref(getTokenMock),
    },
  };
});

const { autocompleteMock } = vi.hoisted(() => ({ autocompleteMock: vi.fn() }));

mockNuxtImport('useAuth', () => () => authState);
// The rows render a `PlayerAvatar`, which has no backend to call here.
mockNuxtImport('useUserService', () => () => ({
  register: () => Promise.resolve(),
  getUserProfile: () => Promise.resolve({ id: 'user', username: 'user', avatar_url: null }),
}));
registerEndpoint('/api/v1/autocomplete/user', (event) =>
  autocompleteMock(event.node.req.headers.authorization),
);

const suggestion = (username: string): UserSuggestion => ({ username, card_count: 42, note: 4.5 });

const search = async (text: string) => {
  const wrapper = await mountSuspended(PlayerPicker, { props: { modelValue: null } });
  const input = wrapper.get('input');
  await input.trigger('focus');
  await input.setValue(text);
  return wrapper;
};

describe('PlayerPicker', () => {
  beforeEach(() => {
    autocompleteMock.mockReset();
    getTokenMock.mockReset();
    getTokenMock.mockResolvedValue('secret-token');
  });

  it('lists the players the autocomplete returns', async () => {
    autocompleteMock.mockResolvedValue([suggestion('alice'), suggestion('alicia')]);
    const wrapper = await search('ali');

    await vi.waitFor(() => expect(wrapper.findAll('[data-prow]')).toHaveLength(2));
    expect(wrapper.text()).toContain('alice');
    expect(wrapper.text()).toContain('2 joueurs');
  });

  it('says no player matches when the autocomplete answers empty', async () => {
    autocompleteMock.mockResolvedValue([]);
    const wrapper = await search('zzz');

    await vi.waitFor(() =>
      expect(wrapper.text()).toContain('Aucun joueur ne correspond à « zzz »'),
    );
    expect(wrapper.find('[data-search-unavailable]').exists()).toBe(false);
  });

  it('says the search is unavailable when the autocomplete fails, not that no player matches', async () => {
    autocompleteMock.mockResolvedValue(new Response(null, { status: 503 }));
    const wrapper = await search('ali');

    await vi.waitFor(() => expect(wrapper.find('[data-search-unavailable]').exists()).toBe(true));
    expect(wrapper.get('[data-search-unavailable]').text()).toContain(
      'Recherche de joueurs indisponible',
    );
    expect(wrapper.text()).not.toContain('Aucun joueur ne correspond');
    expect(wrapper.text()).not.toContain('0 joueurs');
  });

  it('leaves the unavailable state as soon as a later search answers', async () => {
    autocompleteMock.mockResolvedValueOnce(new Response(null, { status: 503 }));
    const wrapper = await search('ali');
    await vi.waitFor(() => expect(wrapper.find('[data-search-unavailable]').exists()).toBe(true));

    autocompleteMock.mockResolvedValue([suggestion('alice')]);
    await wrapper.get('input').setValue('alic');

    await vi.waitFor(() => expect(wrapper.findAll('[data-prow]')).toHaveLength(1));
    expect(wrapper.find('[data-search-unavailable]').exists()).toBe(false);
  });

  it('searches without a bearer token, even for a signed-in player', async () => {
    authState.isSignedIn.value = true;
    autocompleteMock.mockResolvedValue([suggestion('alice')]);
    const wrapper = await search('ali');

    await vi.waitFor(() => expect(wrapper.findAll('[data-prow]')).toHaveLength(1));
    expect(autocompleteMock).toHaveBeenCalledWith(undefined);
    expect(getTokenMock).not.toHaveBeenCalled();
    authState.isSignedIn.value = false;
  });
});
