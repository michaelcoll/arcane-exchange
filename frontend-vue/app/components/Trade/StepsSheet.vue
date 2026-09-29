<script setup lang="ts">
import type { TradeStatus } from '~/utils/trade';

/* « Les étapes d'un échange » en bottom sheet, contenu de `TradeStepsView` côté iOS : les cinq
 * étapes de la machine à états, l'étape courante mise en avant. */
const props = defineProps<{ status: TradeStatus; partner: string }>();

const emit = defineEmits<{ close: [] }>();

/* -1 pour un échange abandonné : aucune étape n'est alors « en cours ». */
const current = computed(() => TRADE_STEPS.findIndex((s) => s.status === props.status));

type StepState = 'done' | 'current' | 'upcoming';

const stateOf = (i: number): StepState => {
  if (current.value < 0 || i > current.value) return 'upcoming';
  return i < current.value ? 'done' : 'current';
};

const nodeClasses: Record<StepState, string> = {
  done: 'border-emerald-500/40 bg-emerald-500/15 text-emerald-700 dark:border-emerald-400/40 dark:bg-emerald-400/15 dark:text-emerald-300',
  current:
    'border-transparent bg-primary text-[var(--on-primary)] shadow-[0_0_0_4px_var(--primary-fill)]',
  upcoming: 'border-[var(--line-2)] bg-slate-400/10 text-[var(--ink-3)]',
};

const scrollLock = useScrollLock(import.meta.client ? document.body : null);

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') emit('close');
};

onMounted(() => {
  scrollLock.value = true;
  window.addEventListener('keydown', onKeydown);
});
onUnmounted(() => {
  scrollLock.value = false;
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div
    class="fixed inset-0 z-[80] animate-[fade_0.2s_ease] bg-black/60 backdrop-blur-sm"
    @click="emit('close')"
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="trade-steps-title"
      class="fixed right-0 bottom-0 left-0 z-[81] max-h-[84vh] animate-[slideup_0.3s_cubic-bezier(0.3,1,0.4,1)] overflow-auto rounded-t-3xl border-t border-slate-300 bg-white px-5 pt-5 pb-[calc(1.25rem+env(safe-area-inset-bottom))] shadow-2xl dark:border-white/15 dark:bg-zinc-900"
      @click.stop
    >
      <div class="mb-3 flex items-center justify-between gap-3">
        <h3 id="trade-steps-title" class="font-display text-base font-semibold tracking-tight">
          Les étapes d'un échange
        </h3>
        <button
          type="button"
          aria-label="Fermer"
          class="grid h-9 w-9 place-items-center rounded-lg border border-slate-200 bg-slate-100 text-slate-600 transition-all duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-800 dark:border-white/10 dark:bg-white/5 dark:text-slate-300 dark:hover:border-white/15 dark:hover:bg-zinc-800 dark:hover:text-slate-100"
          @click="emit('close')"
        >
          <Icon name="lucide:x" :size="16" />
        </button>
      </div>

      <p class="mb-[18px] text-sm text-[var(--ink-2)]">
        <template v-if="current < 0">
          Cet échange a été abandonné avant son terme. Voici les cinq étapes qu'il aurait
          traversées.
        </template>
        <template v-else>
          Un échange avec
          <span v-if="partner" class="text-secondary">@{{ partner }}</span>
          <template v-else>un autre joueur</template>
          traverse cinq étapes. Tu es à l'étape {{ current + 1 }}.
        </template>
      </p>

      <ol>
        <li
          v-for="(step, i) in TRADE_STEPS"
          :key="step.status"
          :data-state="stateOf(i)"
          class="flex gap-[13px]"
        >
          <div class="flex w-[26px] flex-none flex-col items-center gap-[3px]">
            <span
              :class="[
                'grid h-[26px] w-[26px] place-items-center rounded-full border font-mono text-[11px] font-bold',
                nodeClasses[stateOf(i)],
              ]"
            >
              <Icon v-if="stateOf(i) === 'done'" name="lucide:check" :size="13" />
              <template v-else>{{ i + 1 }}</template>
            </span>
            <span
              v-if="i < TRADE_STEPS.length - 1"
              :class="[
                'w-0.5 flex-1 rounded-full',
                stateOf(i) === 'done' ? 'bg-emerald-500/45' : 'bg-slate-400/20',
              ]"
            />
          </div>
          <div
            :class="['flex flex-col gap-1 pt-0.5', i < TRADE_STEPS.length - 1 ? 'pb-[18px]' : '']"
          >
            <span
              :class="[
                'text-[15px] font-semibold',
                stateOf(i) === 'upcoming' ? 'text-[var(--ink-3)]' : 'text-[var(--ink)]',
              ]"
              >{{ step.title }}</span
            >
            <span
              :class="[
                'text-[13px] text-[var(--ink-2)]',
                stateOf(i) === 'upcoming' ? 'opacity-70' : '',
              ]"
              >{{ step.detail }}</span
            >
            <span
              v-if="stateOf(i) === 'current'"
              data-part="current"
              class="border-primary/30 bg-primary/10 text-primary-ink mt-[3px] inline-flex w-fit items-center gap-1.5 rounded-full border px-[9px] py-1 font-mono text-[10px] font-semibold tracking-[0.1em]"
            >
              <Icon name="lucide:clock" :size="11" /> ÉTAPE EN COURS
            </span>
          </div>
        </li>
      </ol>

      <p
        v-if="current < 0"
        class="mt-[18px] flex items-center gap-2 text-[13px] text-[var(--ink-2)]"
      >
        <Icon name="lucide:lock-open" :size="14" /> Les cartes réservées ont été libérées.
      </p>
    </div>
  </div>
</template>
