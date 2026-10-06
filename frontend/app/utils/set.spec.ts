import { describe, it, expect } from 'vitest';
import type { SetInfo } from '~/bindings/SetInfo';
import { resolveSetName } from './set';

const setList: SetInfo[] = [
  { code: 'FDN', name: 'Foundations' },
  { code: 'ACR', name: "Assassin's Creed" },
];

describe('resolveSetName', () => {
  it('returns the full name when the code is present in the set list', () => {
    expect(resolveSetName(setList, 'FDN')).toBe('Foundations');
  });

  it('falls back to the uppercased code when the code is absent from the set list', () => {
    expect(resolveSetName(setList, 'unknown')).toBe('UNKNOWN');
  });

  it('falls back to the uppercased code when the set list is empty', () => {
    expect(resolveSetName([], 'fdn')).toBe('FDN');
  });
});
