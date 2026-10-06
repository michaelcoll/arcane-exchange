// Past this delay Clerk is deemed unreachable (blocked script, network down): the navigation
// goes on to /sign-in rather than hanging forever.
export const CLERK_LOAD_TIMEOUT_MS = 10_000;

export default defineNuxtRouteMiddleware(async (to) => {
  const { isLoaded, isSignedIn } = useAuth();

  // SPA: on a reload the middleware runs before Clerk has loaded, and `isSignedIn` is still
  // `undefined`. Deciding now would send a signed-in user to /sign-in and lose the route.
  await until(isLoaded).toBe(true, { timeout: CLERK_LOAD_TIMEOUT_MS });

  if (!isSignedIn.value) {
    return navigateTo(signInLocation(to.fullPath));
  }
});
