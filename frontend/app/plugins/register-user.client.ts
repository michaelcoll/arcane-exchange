export default defineNuxtPlugin(() => {
  const { isSignedIn, isLoaded } = useAuth();
  const { register } = useUserService();

  watch(
    () => isLoaded.value && isSignedIn.value,
    (signedIn) => {
      // Logged without a toast on purpose: the player can do nothing about a failed registration,
      // and the next sign-in or reload registers again.
      if (signedIn) register().catch((err) => console.error('User registration failed', err));
    },
    { immediate: true },
  );
});
