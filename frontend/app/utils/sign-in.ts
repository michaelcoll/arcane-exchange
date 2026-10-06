export const SIGN_IN_PATH = '/sign-in';

/**
 * Query parameter holding the route to return to once signed in. Clerk's own name: its router
 * keeps it across the steps of `<SignIn>` (`/sign-in/factor-one`, `/sign-in/factor-two`…).
 */
export const SIGN_IN_REDIRECT_PARAM = 'redirect_url';

/** The sign-in page, sending the user back to `returnTo` once signed in. */
export const signInLocation = (returnTo: string) => ({
  path: SIGN_IN_PATH,
  query: { [SIGN_IN_REDIRECT_PARAM]: returnTo },
});
