<script setup lang="ts">
const { clerkAppearance } = useAppTheme();
const redirectUrl = useSignInRedirectUrl();
</script>

<template>
  <div class="mx-auto flex max-w-[1180px] justify-center px-5 pt-16 pb-10">
    <SignIn
      :appearance="clerkAppearance"
      :force-redirect-url="redirectUrl"
      :sign-up-force-redirect-url="redirectUrl"
    />
  </div>
</template>
