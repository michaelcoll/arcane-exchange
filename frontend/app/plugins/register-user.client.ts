export default defineNuxtPlugin(() => {
  const { isSignedIn, isLoaded } = useAuth();
  const { register } = useUserService();

  watch(
    () => isLoaded.value && isSignedIn.value,
    (signedIn) => {
      if (signedIn) register().catch((err) => console.error('User registration failed', err));
    },
    { immediate: true },
  );
});
