import { useRoute, useRouter } from 'nuxt/app';
import type { LocationQuery } from 'vue-router';

/** Whether the current route query already holds exactly `expected`. */
export const isSameQuery = (current: LocationQuery, expected: Record<string, string>) =>
  Object.keys(current).length === Object.keys(expected).length &&
  Object.entries(expected).every(([k, v]) => current[k] === v);

/**
 * Keeps the URL query of the current page equal to `query`, so that a reload (or a bookmark)
 * replays what the page shows. `router.replace`: no history entry per change.
 *
 * The route is watched too: a navigation link to the same page without parameters (which does
 * not remount it) gets the displayed state back into the URL. Call `sync` once the page has read
 * its state back from the URL, so that the invalid or default values it dropped leave the URL.
 */
export const useQuerySync = (query: MaybeRefOrGetter<Record<string, string>>) => {
  const route = useRoute();
  const router = useRouter();
  // The path of the page, read once: outside a page component `route` follows the navigation.
  const { path } = route;

  // A navigation to another page is pending: `currentRoute` shows this page until it settles, and
  // a `router.replace` meanwhile would cancel it.
  let leaving = false;
  const stopBefore = router.beforeEach((to) => {
    leaving = to.path !== path;
  });
  const stopAfter = router.afterEach(() => {
    leaving = false;
  });
  tryOnScopeDispose(() => {
    stopBefore();
    stopAfter();
  });

  const sync = () => {
    // Page being left: don't put its parameters on the next one.
    if (leaving || router.currentRoute.value.path !== path) return;
    const expected = toValue(query);
    if (!isSameQuery(route.query, expected)) router.replace({ path, query: expected });
  };

  watch([() => toValue(query), () => route.query], sync);

  return { sync };
};
