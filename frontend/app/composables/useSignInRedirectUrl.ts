/**
 * Where `<SignIn>` sends the user once signed in: the route set by the `auth` middleware
 * (`signInLocation`) when it is an internal path, `/` otherwise.
 *
 * Passed as `forceRedirectUrl` so it takes precedence over Clerk's own reading of the raw
 * `redirect_url` query parameter.
 */
export const useSignInRedirectUrl = () => {
  const route = useRoute();
  return computed(() => safeRedirectPath(route.query[SIGN_IN_REDIRECT_PARAM]) ?? '/');
};
