import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useToast } from '~/composables/useToast';

describe('useToast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useToast().toasts.value = [];
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts with no toasts', () => {
    expect(useToast().toasts.value).toEqual([]);
  });

  it('pushes an error toast', () => {
    const { toasts, showError } = useToast();
    showError('Action refusée', 'Le trade est vide.');
    expect(toasts.value).toHaveLength(1);
    expect(toasts.value[0]).toMatchObject({
      title: 'Action refusée',
      message: 'Le trade est vide.',
    });
  });

  it('stacks several toasts', () => {
    const { toasts, showError } = useToast();
    showError('Erreur 1', 'a');
    showError('Erreur 2', 'b');
    expect(toasts.value).toHaveLength(2);
    expect(toasts.value.map((t) => t.title)).toEqual(['Erreur 1', 'Erreur 2']);
  });

  it('dismisses a toast by id, leaving the others', () => {
    const { toasts, showError, dismiss } = useToast();
    showError('Erreur 1', 'a');
    showError('Erreur 2', 'b');
    const [first] = toasts.value;
    dismiss(first!.id);
    expect(toasts.value).toHaveLength(1);
    expect(toasts.value[0]!.title).toBe('Erreur 2');
  });

  it('auto-dismisses after 6 seconds', () => {
    const { toasts, showError } = useToast();
    showError('Erreur', 'a');
    expect(toasts.value).toHaveLength(1);
    vi.advanceTimersByTime(6000);
    expect(toasts.value).toHaveLength(0);
  });
});
