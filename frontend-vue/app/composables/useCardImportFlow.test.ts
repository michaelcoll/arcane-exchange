import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useCardImportFlow, type CardImportServices } from './useCardImportFlow';
import type { CardImport } from '~/bindings/CardImport';

function makeImport(overrides: Partial<CardImport> = {}): CardImport {
  return {
    id: 'import-1',
    status: 'pending',
    source_lines: 3,
    total_lines: 3,
    processed_lines: 0,
    errors: [],
    error_count: 0,
    error_message: null,
    created_at: '2026-01-01T00:00:00Z',
    finished_at: null,
    ...overrides,
  };
}

describe('useCardImportFlow', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts idle', () => {
    const services: CardImportServices = {
      importCards: vi.fn(),
      getCardImport: vi.fn(),
      listCardImports: vi.fn(),
    };
    const flow = useCardImportFlow(services);

    expect(flow.step.value).toBe('idle');
    expect(flow.error.value).toBeNull();
  });

  it('follows a successful import from progress to completed and refreshes once', async () => {
    const onCompleted = vi.fn();
    const getCardImport = vi
      .fn()
      .mockResolvedValueOnce(makeImport({ status: 'running', processed_lines: 1 }))
      .mockResolvedValueOnce(makeImport({ status: 'completed', processed_lines: 3 }));
    const services: CardImportServices = {
      importCards: vi.fn().mockResolvedValue({ id: 'import-1' }),
      getCardImport,
      listCardImports: vi.fn(),
    };

    const flow = useCardImportFlow(services, { pollIntervalMs: 1000, onCompleted });

    await flow.start('csv-content');
    await vi.advanceTimersByTimeAsync(0);
    expect(flow.step.value).toBe('progress');
    expect(flow.progressPercent.value).toBe(33);

    await vi.advanceTimersByTimeAsync(1000);

    expect(flow.step.value).toBe('done');
    expect(flow.status.value?.status).toBe('completed');
    expect(flow.progressPercent.value).toBe(100);
    expect(onCompleted).toHaveBeenCalledTimes(1);
    // The interval must be cleared once the import is terminal.
    expect(getCardImport).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(5000);
    expect(getCardImport).toHaveBeenCalledTimes(2);
  });

  it('marks failed imports as done without calling onCompleted', async () => {
    const onCompleted = vi.fn();
    const services: CardImportServices = {
      importCards: vi.fn().mockResolvedValue({ id: 'import-1' }),
      getCardImport: vi
        .fn()
        .mockResolvedValue(
          makeImport({ status: 'failed', error_message: 'no valid line in file' }),
        ),
      listCardImports: vi.fn(),
    };

    const flow = useCardImportFlow(services, { onCompleted });

    await flow.start('csv-content');
    await vi.advanceTimersByTimeAsync(0);

    expect(flow.step.value).toBe('done');
    expect(flow.status.value?.status).toBe('failed');
    expect(onCompleted).not.toHaveBeenCalled();
  });

  it('follows the already-running import on a 409 instead of failing', async () => {
    const activeImport = makeImport({ id: 'active-import', status: 'running' });
    const services: CardImportServices = {
      importCards: vi.fn().mockRejectedValue({ statusCode: 409 }),
      getCardImport: vi.fn().mockResolvedValue(activeImport),
      listCardImports: vi.fn().mockResolvedValue([activeImport]),
    };

    const flow = useCardImportFlow(services);

    await flow.start('csv-content');
    await vi.advanceTimersByTimeAsync(0);

    expect(flow.step.value).toBe('progress');
    expect(flow.wasAlreadyRunning.value).toBe(true);
    expect(flow.status.value?.id).toBe('active-import');
    expect(services.getCardImport).toHaveBeenCalledWith('active-import');
  });

  it('shows an error when a 409 has no active import to follow', async () => {
    const services: CardImportServices = {
      importCards: vi.fn().mockRejectedValue({ statusCode: 409 }),
      getCardImport: vi.fn(),
      listCardImports: vi.fn().mockResolvedValue([]),
    };

    const flow = useCardImportFlow(services);

    await flow.start('csv-content');

    expect(flow.step.value).toBe('idle');
    expect(flow.wasAlreadyRunning.value).toBe(true);
    expect(flow.error.value).toBe('Un import est déjà en cours.');
  });

  it('surfaces the server error message on a non-409 failure', async () => {
    const services: CardImportServices = {
      importCards: vi.fn().mockRejectedValue({ data: { error: 'Body is not valid UTF-8' } }),
      getCardImport: vi.fn(),
      listCardImports: vi.fn(),
    };

    const flow = useCardImportFlow(services);

    await flow.start('csv-content');

    expect(flow.step.value).toBe('idle');
    expect(flow.error.value).toBe('Body is not valid UTF-8');
  });

  it('reset stops polling and clears state', async () => {
    const getCardImport = vi.fn().mockResolvedValue(makeImport({ status: 'running' }));
    const services: CardImportServices = {
      importCards: vi.fn().mockResolvedValue({ id: 'import-1' }),
      getCardImport,
      listCardImports: vi.fn(),
    };

    const flow = useCardImportFlow(services);
    await flow.start('csv-content');
    await vi.advanceTimersByTimeAsync(0);
    expect(flow.step.value).toBe('progress');

    flow.reset();

    expect(flow.step.value).toBe('idle');
    expect(flow.status.value).toBeNull();

    const callsBefore = getCardImport.mock.calls.length;
    await vi.advanceTimersByTimeAsync(5000);
    expect(getCardImport).toHaveBeenCalledTimes(callsBefore);
  });
});
