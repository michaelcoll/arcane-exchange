<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    title: string;
    body: string;
    confirmLabel: string;
    /** `down` pour une action destructrice (modification / abandon), `cyan` sinon. */
    tone?: 'cyan' | 'down';
  }>(),
  { tone: 'cyan' },
);

const emit = defineEmits<{ confirm: []; cancel: [] }>();

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') emit('cancel');
};

onMounted(() => window.addEventListener('keydown', onKeydown));
onUnmounted(() => window.removeEventListener('keydown', onKeydown));

const isDanger = computed(() => props.tone === 'down');
</script>

<template>
  <div
    class="fixed inset-0 z-[80] grid animate-[fade_0.2s_ease] place-items-center bg-black/60 px-5 py-5 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    @click="emit('cancel')"
  >
    <div
      class="w-full max-w-[440px] animate-[pop_0.26s_cubic-bezier(0.3,1.2,0.4,1)] rounded-2xl border border-slate-300 bg-white p-5 shadow-2xl dark:border-white/15 dark:bg-zinc-900"
      @click.stop
    >
      <div class="mb-2.5 flex items-center gap-3">
        <span
          :class="[
            'grid h-[38px] w-[38px] flex-none place-items-center rounded-xl border',
            isDanger
              ? 'border-red-500/40 bg-red-500/10 text-red-600 dark:border-red-400/40 dark:bg-red-400/10 dark:text-red-400'
              : 'border-cyan-500/30 bg-cyan-500/10 text-cyan-600 dark:border-cyan-400/30 dark:bg-cyan-400/10 dark:text-cyan-400',
          ]"
        >
          <Icon :name="isDanger ? 'lucide:triangle-alert' : 'lucide:lock'" size="18" />
        </span>
        <h3 class="font-display text-lg font-semibold tracking-tight">{{ title }}</h3>
      </div>
      <p class="text-sm leading-relaxed text-slate-600 dark:text-slate-300">{{ body }}</p>
      <div class="mt-4 flex flex-wrap items-center justify-end gap-2.5">
        <button
          class="inline-flex items-center justify-center gap-2 rounded-xl border border-slate-200 bg-transparent px-4 py-2.5 text-sm leading-none font-semibold whitespace-nowrap text-slate-600 transition-all duration-150 hover:-translate-y-px hover:border-slate-300 hover:bg-slate-100 hover:text-slate-800 active:translate-y-0 dark:border-white/10 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-white/5 dark:hover:text-slate-100"
          @click="emit('cancel')"
        >
          Annuler
        </button>
        <button
          :class="[
            'inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm leading-none font-bold whitespace-nowrap transition-all duration-150 hover:-translate-y-px active:translate-y-0',
            isDanger
              ? 'border border-red-500/40 bg-transparent text-red-600 hover:bg-red-500/10 dark:border-red-400/40 dark:text-red-400 dark:hover:bg-red-400/10'
              : 'border border-transparent bg-cyan-500 text-zinc-950 shadow-lg hover:bg-cyan-400 dark:bg-cyan-400 dark:hover:bg-cyan-300',
          ]"
          @click="emit('confirm')"
        >
          {{ confirmLabel }}
        </button>
      </div>
    </div>
  </div>
</template>
