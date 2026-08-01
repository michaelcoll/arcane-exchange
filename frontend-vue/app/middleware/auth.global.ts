const isProtectedRoute = createRouteMatcher([
  '/collection(.*)',
  '/trade(.*)',
  '/search(.*)',
  '/profile(.*)',
]);

export default defineNuxtRouteMiddleware((to) => {
  const { isSignedIn } = useAuth();

  if (!isSignedIn.value && isProtectedRoute(to)) {
    return navigateTo('/sign-in');
  }
});
