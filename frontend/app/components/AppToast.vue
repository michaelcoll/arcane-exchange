<script setup lang="ts">
const { toasts, dismiss } = useToast();
</script>

<template>
  <div
    class="pointer-events-none fixed inset-x-0 bottom-[calc(20px+env(safe-area-inset-bottom))] z-[120] flex flex-col items-center gap-2 px-4 max-md:bottom-[calc(78px+env(safe-area-inset-bottom))]"
  >
    <div
      v-for="t in toasts"
      :key="t.id"
      role="alert"
      class="toast-pop pointer-events-auto flex w-[min(440px,100%)] items-center gap-3 rounded-2xl border border-red-500/40 bg-white px-3.5 py-3 shadow-2xl dark:border-red-400/40 dark:bg-zinc-900"
    >
      <span class="flex-none text-red-600 dark:text-red-400">
        <Icon name="lucide:alert-triangle" size="17" />
      </span>
      <div class="flex min-w-0 flex-1 flex-col gap-px">
        <span class="text-[13.5px] font-semibold text-slate-800 dark:text-slate-100">{{
          t.title
        }}</span>
        <span class="text-xs text-slate-600 dark:text-slate-300">{{ t.message }}</span>
      </div>
      <button
        class="flex-none rounded-lg p-1 text-slate-400 transition-colors duration-150 hover:bg-slate-100 hover:text-slate-800 dark:text-slate-500 dark:hover:bg-white/5 dark:hover:text-slate-100"
        aria-label="Fermer"
        @click="dismiss(t.id)"
      >
        <Icon name="lucide:x" size="15" />
      </button>
    </div>
  </div>
</template>
