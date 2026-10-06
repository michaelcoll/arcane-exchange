import { useRoute, useRouter } from 'nuxt/app';

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

  const sync = () => {
    // Page being left: don't put its parameters on the next one.
    if (router.currentRoute.value.path !== route.path) return;
    const expected = toValue(query);
    if (!isSameQuery(route.query, expected)) router.replace({ path: route.path, query: expected });
  };

  watch([() => toValue(query), () => route.query], sync);

  return { sync };
};
