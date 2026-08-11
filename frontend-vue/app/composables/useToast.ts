export interface Toast {
  id: number;
  title: string;
  message: string;
}

const TOAST_DURATION_MS = 6000;
let nextId = 0;

export const useToast = () => {
  const toasts = useState<Toast[]>('toasts', () => []);

  const dismiss = (id: number) => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  };

  const showError = (title: string, message: string) => {
    const id = nextId++;
    toasts.value = [...toasts.value, { id, title, message }];
    setTimeout(() => dismiss(id), TOAST_DURATION_MS);
  };

  return { toasts, showError, dismiss };
};
