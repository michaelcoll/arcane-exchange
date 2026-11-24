import type { MaybeRefOrGetter } from 'vue';

// Session-shared avatar cache keyed by username: one GET /user/{username} per player, even in
// lists. Loading only starts on client mount (onMounted), never during SSR, so the module-level
// cache stays empty on the server and cannot leak between user sessions.
const avatars = new Map<string, string | null>();
const inFlight = new Map<string, Promise<string | null>>();

export const useUserAvatar = (username: MaybeRefOrGetter<string>) => {
  const target = computed(() => toValue(username));
  const avatar = ref<string | null>(null);
  const { getUserProfile } = useUserService();

  const load = async () => {
    const u = target.value;
    if (avatars.has(u)) {
      avatar.value = avatars.get(u) ?? null;
      return;
    }

    let promise = inFlight.get(u);
    if (!promise) {
      promise = getUserProfile(u)
        .then((profile) => profile.avatar_url)
        .catch((error: unknown) => {
          // Only a real 404 (profile without avatar or not found) is cached: a transient error
          // (network, 5xx, expired token) must not freeze the user on initials for the whole
          // session.
          if (readApiError(error).status === 404) {
            return null;
          }
          throw error;
        });
      inFlight.set(u, promise);
    }

    let result: string | null;
    try {
      result = await promise;
    } catch {
      // Ignored on purpose: a transient error leaves the initials and caches nothing, so the next
      // load retries.
      inFlight.delete(u);
      return;
    }
    inFlight.delete(u);
    avatars.set(u, result);
    // The target changed during the request: stale result, do not display it.
    if (target.value !== u) return;
    avatar.value = result;
  };

  // Player change: go back to initials, then load the new avatar. Otherwise the previous
  // player's avatar would stay displayed during the request, and for good if it fails.
  watch(target, () => {
    avatar.value = null;
    load();
  });

  return { avatar, load };
};
