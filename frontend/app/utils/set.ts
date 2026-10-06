import type { SetInfo } from '~/bindings/SetInfo';

export const resolveSetName = (setList: SetInfo[], code: string): string =>
  setList.find((s) => s.code === code)?.name ?? code.toUpperCase();
