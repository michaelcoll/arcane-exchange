import { describe, it, expect } from 'vitest';
import { safeRedirectPath } from './safe-redirect';

describe('safeRedirectPath', () => {
  it.each(['/', '/trade/42', '/search?q=Sol%20Ring&mode=name', '/collection#top'])(
    'keeps the internal path %s',
    (path) => {
      expect(safeRedirectPath(path)).toBe(path);
    },
  );

  it.each([
    ['a protocol-relative URL', '//evil.example'],
    ['a backslash protocol-relative URL', '/\\evil.example'],
    ['an absolute URL', 'https://evil.example/trade'],
    ['a javascript: URL', 'javascript:alert(1)'],
    ['a path without leading slash', 'trade/42'],
    ['a path hiding a control character', '/\t/evil.example'],
    ['an empty string', ''],
  ])('rejects %s', (_label, value) => {
    expect(safeRedirectPath(value)).toBeNull();
  });

  it.each([undefined, null, ['/trade'], 42])('rejects the non-string value %j', (value) => {
    expect(safeRedirectPath(value)).toBeNull();
  });
});
