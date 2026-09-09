import { computed, ref } from 'vue';
import type { CardImport } from '~/bindings/CardImport';
import type { CardImportStarted } from '~/bindings/CardImportStarted';

export interface CardImportServices {
  importCards: (csv: string) => Promise<CardImportStarted>;
  getCardImport: (id: string) => Promise<CardImport>;
  listCardImports: () => Promise<CardImport[]>;
}

export type CardImportStep = 'idle' | 'progress' | 'done';

export interface CardImportFlowOptions {
  /** Defaults to 1000ms — override in tests to avoid waiting on real timers. */
  pollIntervalMs?: number;
  /** Called once, right when the import reaches `completed`. */
  onCompleted?: () => void;
}

/**
 * Drives an async card import from submission to completion: posts the CSV, then polls
 * `GET /collection/import/{id}` until a terminal state is reached. A `409` (an import is
 * already running for this user) switches to following that import instead of failing outright.
 */
export const useCardImportFlow = (
  services: CardImportServices,
  options: CardImportFlowOptions = {},
) => {
  const pollIntervalMs = options.pollIntervalMs ?? 1000;

  const step = ref<CardImportStep>('idle');
  const status = ref<CardImport | null>(null);
  const error = ref<string | null>(null);
  const wasAlreadyRunning = ref(false);
  let pollHandle: ReturnType<typeof setInterval> | null = null;

  const progressPercent = computed(() => {
    const total = status.value?.total_lines ?? 0;
    const processed = status.value?.processed_lines ?? 0;
    if (total <= 0) return status.value?.status === 'completed' ? 100 : 0;
    return Math.min(100, Math.round((processed / total) * 100));
  });

  const stopPolling = () => {
    if (pollHandle) {
      clearInterval(pollHandle);
      pollHandle = null;
    }
  };

  const reset = () => {
    stopPolling();
    step.value = 'idle';
    status.value = null;
    error.value = null;
    wasAlreadyRunning.value = false;
  };

  const poll = (id: string) => {
    stopPolling();
    step.value = 'progress';

    const tick = async () => {
      try {
        const current = await services.getCardImport(id);
        status.value = current;
        if (current.status === 'completed' || current.status === 'failed') {
          stopPolling();
          step.value = 'done';
          if (current.status === 'completed') options.onCompleted?.();
        }
      } catch {
        stopPolling();
        step.value = 'idle';
        error.value = "Erreur lors du suivi de l'import";
      }
    };

    // First check right away — don't make the user wait a full interval to see it start.
    void tick();
    pollHandle = setInterval(tick, pollIntervalMs);
  };

  const start = async (csv: string): Promise<void> => {
    error.value = null;
    wasAlreadyRunning.value = false;

    try {
      const started = await services.importCards(csv);
      poll(started.id);
    } catch (e: unknown) {
      const err = e as {
        statusCode?: number;
        response?: { status?: number };
        data?: { error?: string };
      };
      if (err?.statusCode === 409 || err?.response?.status === 409) {
        wasAlreadyRunning.value = true;
        const imports = await services.listCardImports().catch(() => []);
        const active = imports.find((i) => i.status === 'pending' || i.status === 'running');
        if (active) {
          poll(active.id);
        } else {
          error.value = 'Un import est déjà en cours.';
        }
      } else {
        error.value = err?.data?.error ?? "Erreur lors de l'import";
      }
    }
  };

  return {
    step,
    status,
    error,
    wasAlreadyRunning,
    progressPercent,
    start,
    stopPolling,
    reset,
  };
};
