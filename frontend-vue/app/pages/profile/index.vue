<script setup lang="ts">
definePageMeta({ middleware: 'auth' });

const { user } = useUser();

const initials = computed(() => {
  if (!user.value) return '?';
  const fn = user.value.firstName?.[0] ?? '';
  const ln = user.value.lastName?.[0] ?? '';
  return (fn + ln).toUpperCase() || user.value.username?.[0]?.toUpperCase() || '?';
});

const contact = computed(
  () => user.value?.primaryEmailAddress?.emailAddress ?? user.value?.username ?? '',
);

const vis = ref<'public' | 'trade' | 'private'>('trade');
const manageOpen = ref(false);

const bodyScrollLocked = useScrollLock(document.body);
watch(manageOpen, (open) => {
  bodyScrollLocked.value = open;
});

const visOptions = [
  { value: 'public', label: 'Publique', tone: 'cyan' },
  { value: 'trade', label: 'Pour échange', tone: 'cyan' },
  { value: 'private', label: 'Privée', tone: 'cyan' },
];

const visHelp = [
  {
    value: 'public',
    label: 'Publique',
    text: 'toute ta collection est visible, y compris les cartes que tes règles ne proposent pas.',
  },
  {
    value: 'trade',
    label: 'Pour échange',
    text: 'seules les cartes retenues par tes règles ci-dessous apparaissent. Le reste reste privé.',
  },
  {
    value: 'private',
    label: 'Privée',
    text: "personne ne voit ta collection et tu n'apparais dans aucune recherche.",
  },
];
</script>

<template>
  <div class="mx-auto max-w-[680px] px-5 pt-7 pb-10 max-md:px-4 max-md:pt-5 max-md:pb-8">
    <h2 class="font-display mb-4 text-xl font-semibold tracking-tight">Préférences</h2>

    <!-- COMPTE -->
    <div
      class="mb-6 rounded-2xl border border-slate-200 bg-white/60 p-4 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
    >
      <div class="flex items-center justify-between gap-3.5">
        <div class="flex items-center gap-3">
          <div
            class="relative grid h-8 w-8 shrink-0 place-items-center overflow-hidden rounded-full border border-slate-300 bg-slate-100 font-mono text-xs text-slate-600 dark:border-white/15 dark:bg-zinc-800 dark:text-slate-300"
          >
            <img
              v-if="user?.imageUrl"
              :src="user.imageUrl"
              :alt="user.fullName ?? ''"
              class="h-full w-full rounded-full object-cover"
            />
            <template v-else>{{ initials }}</template>
          </div>
          <div class="flex flex-col gap-0.5">
            <span
              class="overflow-hidden text-base font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
              >{{ user?.fullName ?? user?.username ?? '—' }}</span
            >
            <span class="flex items-center gap-1.5 text-xs text-slate-400 dark:text-slate-500">
              {{ contact }} ·
              <span class="inline-flex items-center gap-1 text-violet-500 dark:text-violet-300">
                <Icon name="lucide:shield" size="12" /> géré par Clerk
              </span>
            </span>
          </div>
        </div>
        <button
          class="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-200 bg-transparent px-3 py-1.5 text-xs leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
          @click="manageOpen = true"
        >
          Gérer le compte <Icon name="lucide:arrow-up-right" size="14" />
        </button>
      </div>
    </div>

    <!-- MODAL CLERK USER PROFILE -->
    <div
      v-if="manageOpen"
      class="fixed inset-0 z-[80] grid animate-[fade_0.2s_ease] place-items-center bg-black/60 p-5 backdrop-blur-sm"
      @click.self="manageOpen = false"
    >
      <div class="overflow-hidden rounded-3xl p-0" @click.stop>
        <UserProfile />
      </div>
    </div>

    <!-- CONFIDENTIALITÉ -->
    <section class="mb-6">
      <span
        class="text-2xs mb-3 block font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Confidentialité de la collection</span
      >
      <div
        class="rounded-2xl border border-slate-200 bg-white/60 p-1.5 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div
          class="flex items-center justify-between gap-4 px-3.5 pt-3.5 pb-1.5 max-sm:flex-col max-sm:items-stretch max-sm:gap-3"
        >
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-semibold">Visibilité de la collection</span>
            <span class="text-xs text-slate-400 dark:text-slate-500"
              >Qui peut voir tes cartes pour proposer un échange</span
            >
          </div>
          <SegToggle v-model="vis" :options="visOptions" size="sm" class="max-sm:self-start" />
        </div>

        <div class="mx-3.5 mb-3 flex flex-col gap-0.5 pl-0.5">
          <div
            v-for="o in visHelp"
            :key="o.value"
            :class="[
              'grid grid-cols-[auto_1fr] items-baseline gap-2.5 rounded-lg py-1.5 pr-2.5 pl-1.5 text-xs leading-relaxed transition-all duration-200',
              vis === o.value
                ? 'bg-slate-100 text-slate-600 dark:bg-white/5 dark:text-slate-300'
                : 'text-slate-400 dark:text-slate-500',
            ]"
          >
            <span
              :class="[
                'block h-[5px] w-[5px] -translate-y-0.5 rounded-full transition-all duration-200',
                vis === o.value
                  ? 'bg-cyan-500 shadow-[0_0_8px_-1px_var(--cyan-glow)] dark:bg-cyan-400'
                  : 'bg-slate-300 dark:bg-zinc-600',
              ]"
            />
            <span>
              <b
                :class="[
                  'font-semibold transition-colors duration-200',
                  vis === o.value
                    ? 'text-cyan-600 dark:text-cyan-400'
                    : 'text-slate-500 dark:text-slate-400',
                ]"
                >{{ o.label }}</b
              >
              — {{ o.text }}
            </span>
          </div>
        </div>
      </div>
    </section>

    <!-- RÈGLES DE MISE À L'ÉCHANGE -->
    <ProfileTradeRules v-if="vis === 'trade'" />
  </div>
</template>
