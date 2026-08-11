<script setup lang="ts">
const { theme, themePreference, cycleThemePreference, clerkAppearance } = useAppTheme();

if (import.meta.client) {
  const saved = localStorage.getItem('tae_theme');
  if (saved === 'light' || saved === 'dark' || saved === 'auto') themePreference.value = saved;
}

const htmlClass = computed(() => (theme.value === 'dark' ? 'dark' : ''));

useHead({
  htmlAttrs: { class: htmlClass },
  bodyAttrs: { 'data-bg': 'aurora' },
});

watch(themePreference, (val) => {
  if (import.meta.client) localStorage.setItem('tae_theme', val);
});

const themeToggleIcon = computed(
  () =>
    ({ auto: 'lucide:sun-moon', light: 'lucide:sun', dark: 'lucide:moon' })[themePreference.value],
);

const themeToggleLabel = computed(
  () =>
    ({
      auto: 'Thème automatique (suit le système)',
      light: 'Mode clair',
      dark: 'Mode sombre',
    })[themePreference.value],
);

const { isSignedIn, isLoaded } = useAuth();

const route = useRoute();
const isActive = (path: string) => route.path === path;

const navLinkClass = (path: string) => [
  'relative flex items-center px-4 text-sm font-medium transition-[color,background] duration-200 whitespace-nowrap',
  isActive(path)
    ? 'text-cyan-600 dark:text-cyan-400 bg-cyan-500/10 dark:bg-cyan-400/10 shadow-[inset_0_-2px_0_currentColor]'
    : 'text-slate-600 dark:text-slate-300 hover:text-slate-800 dark:hover:text-slate-100 hover:bg-slate-100 dark:hover:bg-white/5',
];

const bottomNavLinkClass = (path: string) => [
  'flex-1 flex flex-col items-center gap-1 text-2xs p-1 font-semibold font-mono tracking-wide transition-colors duration-150',
  isActive(path) ? 'text-cyan-600 dark:text-cyan-400' : 'text-slate-400 dark:text-slate-500',
];
</script>

<template>
  <div class="min-h-screen pb-24">
    <NuxtRouteAnnouncer />

    <!-- HEADER -->
    <header
      class="sticky top-0 z-40 border-b border-slate-200 bg-slate-100/60 pt-[env(safe-area-inset-top)] backdrop-blur-md dark:border-white/10 dark:bg-zinc-950/60"
    >
      <div class="mx-auto flex h-16 max-w-[1180px] items-center gap-4 px-5 max-md:h-14 max-md:px-4">
        <NuxtLink to="/" class="font-display flex cursor-pointer items-center gap-2.5">
          <span
            class="grid h-8 w-8 shrink-0 place-items-center overflow-hidden rounded-lg border border-cyan-500/30 bg-cyan-500/15 dark:border-cyan-400/30 dark:bg-cyan-400/15"
          >
            <svg
              viewBox="0 0 28 28"
              class="h-4 w-4"
              fill="none"
              stroke-width="2.2"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <rect
                x="5"
                y="8.5"
                width="11"
                height="11"
                rx="3"
                transform="rotate(45 10.5 14)"
                stroke="var(--cyan)"
              />
              <rect
                x="12"
                y="8.5"
                width="11"
                height="11"
                rx="3"
                transform="rotate(45 17.5 14)"
                stroke="var(--violet)"
              />
            </svg>
          </span>
          <span class="text-base font-semibold tracking-tight"
            >Arcane <b class="font-semibold text-cyan-600 dark:text-cyan-400">Exchange</b></span
          >
        </NuxtLink>

        <nav v-if="isLoaded && isSignedIn" class="ml-2 flex gap-0.5 self-stretch max-md:hidden">
          <NuxtLink to="/collection" :class="navLinkClass('/collection')">Collection</NuxtLink>
          <NuxtLink to="/trade" :class="navLinkClass('/trade')">Échanges</NuxtLink>
          <NuxtLink to="/search" :class="navLinkClass('/search')">Rechercher</NuxtLink>
          <NuxtLink to="/profile" :class="navLinkClass('/profile')">Profil</NuxtLink>
        </nav>

        <span class="flex-1" />

        <template v-if="isLoaded">
          <NuxtLink
            v-if="!isSignedIn"
            to="/sign-in"
            class="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-200 bg-transparent px-3 py-1.5 text-xs leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
            >Connexion
          </NuxtLink>
        </template>

        <button
          class="grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
          @click="cycleThemePreference"
          :aria-label="themeToggleLabel"
          :title="themeToggleLabel"
        >
          <Icon :name="themeToggleIcon" size="17" />
        </button>

        <button
          class="grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 max-md:hidden dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
          aria-label="Notifications"
        >
          <Icon name="lucide:bell" size="17" />
        </button>

        <div v-if="isLoaded && isSignedIn" class="user-btn-wrap flex h-9 items-center">
          <UserButton :appearance="clerkAppearance" />
        </div>
      </div>
    </header>

    <!-- PAGE CONTENT -->
    <main>
      <NuxtPage />
    </main>

    <AppToast />

    <!-- MOBILE BOTTOM NAV -->
    <nav
      v-if="isLoaded && isSignedIn"
      class="hidden max-md:fixed max-md:right-0 max-md:bottom-0 max-md:left-0 max-md:z-50 max-md:flex max-md:border-t max-md:border-slate-200 max-md:bg-slate-100/80 max-md:px-2 max-md:pt-2 max-md:pb-[calc(0.5rem+env(safe-area-inset-bottom))] max-md:backdrop-blur-md dark:max-md:border-white/10 dark:max-md:bg-zinc-950/80"
    >
      <NuxtLink to="/collection" :class="bottomNavLinkClass('/collection')">
        <Icon name="lucide:layers" size="21" />
        Collection
      </NuxtLink>
      <NuxtLink to="/trade" :class="bottomNavLinkClass('/trade')">
        <Icon name="lucide:arrow-left-right" size="21" />
        Échanges
      </NuxtLink>
      <NuxtLink to="/search" :class="bottomNavLinkClass('/search')">
        <Icon name="lucide:search" size="21" />
        Rechercher
      </NuxtLink>
      <NuxtLink to="/pages/profile" :class="bottomNavLinkClass('/profile')">
        <Icon name="lucide:user" size="21" />
        Profil
      </NuxtLink>
    </nav>
  </div>
</template>

<style>
/* Clerk UserButton internal overrides */
.user-btn-wrap .cl-rootBox,
.user-btn-wrap .cl-userButtonBox {
  display: flex;
  align-items: center;
  height: 34px;
}

.user-btn-wrap .cl-userButtonTrigger {
  width: 34px;
  height: 34px;
  padding: 0;
  border-radius: 50%;
}

.cl-userButtonTrigger.cl-open,
.cl-userButtonTrigger:focus-visible {
  box-shadow:
    0 0 0 2px var(--bg),
    0 0 0 4px var(--cyan) !important;
}
</style>
