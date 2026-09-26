export default defineNuxtRouteMiddleware(async (to) => {
  const { isLoaded, isSignedIn } = useAuth();

  // SPA: on a reload the middleware runs before Clerk has loaded, and `isSignedIn` is still
  // `undefined`. Deciding now would send a signed-in user to /sign-in and lose the route.
  await until(isLoaded).toBe(true);

  if (!isSignedIn.value) {
    return navigateTo({ path: '/sign-in', query: { redirect_url: to.fullPath } });
  }
});
