<script setup lang="ts">
const { user } = useUser();

const initials = computed(() => {
  if (!user.value) return '?';
  const fn = user.value.firstName?.[0] ?? '';
  const ln = user.value.lastName?.[0] ?? '';
  return (fn + ln).toUpperCase() || user.value.username?.[0]?.toUpperCase() || '?';
});

const vis = ref<'public' | 'trade' | 'private'>('trade');
const showVal = ref<'me' | 'all'>('me');
const proximity = ref(true);
const accent = ref('#00daf3');
const manageOpen = ref(false);

const bodyScrollLocked = useScrollLock(document.body);
watch(manageOpen, (open) => {
  bodyScrollLocked.value = open;
});

const THEMES = [
  { c: '#00daf3', name: 'Cyan néon', sub: 'Défaut' },
  { c: '#00f0c8', name: 'Turquoise', sub: '#00F0C8' },
  { c: '#3aa0ff', name: 'Azur', sub: '#3AA0FF' },
  { c: '#19c4e6', name: 'Cyan profond', sub: '#19C4E6' },
];

const setAccent = (c: string) => {
  accent.value = c;
  if (import.meta.client) {
    document.documentElement.style.setProperty('--accent', c);
    localStorage.setItem('tae_accent', c);
  }
};

onMounted(() => {
  if (import.meta.client) {
    const saved = localStorage.getItem('tae_accent');
    if (saved) {
      accent.value = saved;
      document.documentElement.style.setProperty('--accent', saved);
    }
  }
});

const channels = [
  { icon: 'simple-icons:discord', name: 'Discord', value: 'theo_arcaniste#4821', connected: true },
  { icon: 'simple-icons:telegram', name: 'Telegram', value: '@theo_mtg', connected: true },
  { icon: 'simple-icons:whatsapp', name: 'WhatsApp', value: '', connected: false },
];

const visOptions = [
  { value: 'public', label: 'Publique', tone: 'cyan' },
  { value: 'trade', label: 'Pour échange', tone: 'cyan' },
  { value: 'private', label: 'Privée', tone: 'cyan' },
];
const showValOptions = [
  { value: 'me', label: 'Moi seul', tone: 'cyan' },
  { value: 'all', label: 'Tout le monde', tone: 'cyan' },
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
              {{ user?.username ?? '' }} ·
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

    <!-- APPARENCE -->
    <section class="mb-6">
      <div class="mb-3 flex flex-col gap-1.5">
        <span
          class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
          >Apparence · thème d'accent</span
        >
        <span class="text-xs text-slate-400 dark:text-slate-500"
          >La couleur d'action de toute l'app — boutons, valeurs en hausse, états actifs.</span
        >
      </div>
      <div class="grid [grid-template-columns:repeat(auto-fit,minmax(146px,1fr))] gap-2.5">
        <button
          v-for="th in THEMES"
          :key="th.c"
          :data-on="accent === th.c"
          :style="{ '--tc': th.c } as any"
          class="relative flex items-center gap-3 rounded-xl border border-slate-200 bg-white px-3 py-3 text-left transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-50 data-[on=true]:border-[var(--tc)] data-[on=true]:shadow-[inset_0_0_0_1px_var(--tc)] dark:border-white/10 dark:bg-zinc-900 dark:hover:border-white/15 dark:hover:bg-zinc-800"
          @click="setAccent(th.c)"
        >
          <span class="h-8 w-8 shrink-0 rounded-lg bg-[var(--tc)] shadow-inner" />
          <div class="flex flex-col gap-px">
            <span class="text-sm font-semibold text-slate-800 dark:text-slate-100">{{
              th.name
            }}</span>
            <span class="font-mono text-xs text-slate-400 dark:text-slate-500">{{ th.sub }}</span>
          </div>
          <span v-if="accent === th.c" class="ml-auto flex text-[var(--tc)]">
            <Icon name="lucide:check" size="17" />
          </span>
        </button>
      </div>
    </section>

    <!-- CANAUX DE CONTACT -->
    <section class="mb-6">
      <div class="mb-3 flex flex-col gap-1.5">
        <span
          class="text-2xs font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
          >Canaux de contact pour les échanges</span
        >
        <span class="text-xs text-slate-400 dark:text-slate-500"
          >Ces canaux alimentent le bouton « Contacter » de tes transactions.</span
        >
      </div>
      <div class="flex flex-col gap-2">
        <div
          v-for="ch in channels"
          :key="ch.name"
          class="flex items-center gap-3 rounded-xl border border-slate-200 bg-white px-3.5 py-3 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 dark:border-white/10 dark:bg-zinc-900 dark:hover:border-white/15 dark:hover:bg-zinc-800"
        >
          <span
            :class="
              ch.connected
                ? 'text-cyan-600 dark:text-cyan-400'
                : 'text-slate-400 dark:text-slate-500'
            "
            class="grid w-8 shrink-0 place-items-center"
          >
            <Icon :name="ch.icon" size="19" />
          </span>
          <div class="min-w-0 flex-1">
            <div
              class="overflow-hidden text-sm font-semibold text-ellipsis whitespace-nowrap text-slate-800 dark:text-slate-100"
            >
              {{ ch.name }}
            </div>
            <div
              v-if="ch.connected"
              class="flex flex-wrap items-center gap-2 font-mono text-xs tracking-tight text-slate-600 dark:text-slate-300"
            >
              {{ ch.value }}
            </div>
            <div
              v-else
              class="flex flex-wrap items-center gap-2 text-xs text-slate-400 dark:text-slate-500"
            >
              Non connecté
            </div>
          </div>
          <button
            v-if="ch.connected"
            class="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-200 bg-transparent px-3 py-1.5 text-xs leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
          >
            Modifier
          </button>
          <button
            v-else
            class="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-300 bg-slate-100 px-3 py-1.5 text-xs leading-none font-semibold whitespace-nowrap text-slate-800 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-200 active:translate-y-0 dark:border-white/15 dark:bg-zinc-800 dark:text-slate-100 dark:hover:border-white/15 dark:hover:bg-zinc-700"
          >
            <Icon name="lucide:plus" size="14" /> Ajouter
          </button>
        </div>
      </div>
    </section>

    <!-- CONFIDENTIALITÉ -->
    <section>
      <span
        class="text-2xs mb-3 block font-mono font-medium tracking-widest whitespace-nowrap text-slate-400 uppercase dark:text-slate-500"
        >Confidentialité de la collection</span
      >
      <div
        class="rounded-2xl border border-slate-200 bg-white/60 p-1.5 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-zinc-900/60"
      >
        <div class="flex flex-col gap-0">
          <div class="flex items-center justify-between gap-4 px-3.5 py-3.5">
            <div class="flex flex-col gap-0.5">
              <span class="text-sm font-semibold">Visibilité de la collection</span>
              <span class="text-xs text-slate-400 dark:text-slate-500"
                >Qui peut voir tes cartes pour proposer un échange</span
              >
            </div>
            <SegToggle v-model="vis" :options="visOptions" size="sm" />
          </div>

          <div class="h-px bg-slate-200 dark:bg-white/10" />

          <div class="flex items-center justify-between gap-4 px-3.5 py-3.5">
            <div class="flex flex-col gap-0.5">
              <span class="text-sm font-semibold">Afficher la valeur estimée</span>
              <span class="text-xs text-slate-400 dark:text-slate-500"
                >Montant total visible sur ton profil public</span
              >
            </div>
            <SegToggle v-model="showVal" :options="showValOptions" size="sm" />
          </div>

          <div class="h-px bg-slate-200 dark:bg-white/10" />

          <div class="flex items-center justify-between gap-4 px-3.5 py-3.5">
            <div class="flex flex-col gap-0.5">
              <span class="text-sm font-semibold">Apparaître dans les recherches de proximité</span>
              <span class="text-xs text-slate-400 dark:text-slate-500"
                >Les joueurs proches peuvent te trouver par carte ou decklist</span
              >
            </div>
            <button
              :aria-pressed="proximity"
              :class="[
                'relative h-7 w-12 shrink-0 cursor-pointer rounded-full transition-all duration-200',
                proximity
                  ? 'border border-transparent bg-cyan-500 dark:bg-cyan-400'
                  : 'border border-slate-300 bg-slate-200 dark:border-white/15 dark:bg-zinc-800',
              ]"
              @click="proximity = !proximity"
            >
              <span
                :class="[
                  'absolute top-1 h-5 w-5 rounded-full transition-[left] duration-200 ease-out',
                  proximity ? 'left-6 bg-zinc-950' : 'left-1 bg-slate-500 dark:bg-slate-400',
                ]"
              />
            </button>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
