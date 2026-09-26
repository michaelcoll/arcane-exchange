import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import { mockNuxtImport } from '@nuxt/test-utils/runtime';
import { ref, type Ref } from 'vue';
import type { RouteLocationNormalized } from 'vue-router';
import auth from '~/middleware/auth';

// `useAuth` is also called by app.vue and the register-user plugin while the Nuxt test app
// boots, so the mocked state must hold refs from the start.
const { authState, navigateToMock } = await vi.hoisted(async () => {
  const { ref } = await import('vue');
  return {
    authState: {
      isLoaded: ref(false) as Ref<boolean>,
      isSignedIn: ref<boolean | undefined>(undefined),
    },
    navigateToMock: vi.fn(),
  };
});

mockNuxtImport('useAuth', () => () => authState);
mockNuxtImport('navigateTo', () => navigateToMock);

const route = (fullPath: string) => ({ fullPath }) as RouteLocationNormalized;

const runMiddleware = (to: RouteLocationNormalized) => auth(to, route('/'));

describe('auth middleware', () => {
  beforeEach(() => {
    authState.isLoaded = ref(false);
    authState.isSignedIn = ref<boolean | undefined>(undefined);
    navigateToMock.mockReset();
  });

  it('does not redirect while Clerk is loading, then keeps a signed-in user on the route', async () => {
    let settled = false;
    const result = Promise.resolve(runMiddleware(route('/trade/42'))).then((r) => {
      settled = true;
      return r;
    });

    await flushPromises();
    expect(settled).toBe(false);
    expect(navigateToMock).not.toHaveBeenCalled();

    authState.isSignedIn.value = true;
    authState.isLoaded.value = true;

    expect(await result).toBeUndefined();
    expect(navigateToMock).not.toHaveBeenCalled();
  });

  it('sends a signed-out user to /sign-in with the original route', async () => {
    authState.isLoaded.value = true;
    authState.isSignedIn.value = false;

    await runMiddleware(route('/search?q=Sol%20Ring&mode=name'));

    expect(navigateToMock).toHaveBeenCalledWith({
      path: '/sign-in',
      query: { redirect_url: '/search?q=Sol%20Ring&mode=name' },
    });
  });

  it('lets a signed-in user through', async () => {
    authState.isLoaded.value = true;
    authState.isSignedIn.value = true;

    expect(await runMiddleware(route('/collection'))).toBeUndefined();
    expect(navigateToMock).not.toHaveBeenCalled();
  });
});
