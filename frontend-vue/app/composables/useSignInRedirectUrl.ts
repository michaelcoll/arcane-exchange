/**
 * Where `<SignIn>` sends the user once signed in: the `redirect_url` set by the `auth`
 * middleware when it is an internal path, `/` otherwise.
 *
 * Passed as `forceRedirectUrl` so it takes precedence over Clerk's own reading of the raw
 * `redirect_url` query parameter.
 */
export const useSignInRedirectUrl = () => {
  const route = useRoute();
  return computed(() => safeRedirectPath(route.query.redirect_url) ?? '/');
};
