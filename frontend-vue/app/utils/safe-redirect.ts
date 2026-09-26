/**
 * Returns `value` when it is an internal path of this app, `null` otherwise.
 *
 * Only a relative path starting with a single `/` is accepted: `//host` and `/\host` are
 * protocol-relative URLs to another origin for browsers, and anything with a scheme
 * (`https:`, `javascript:`) is external. Guards the `redirect_url` of the sign-in page
 * against open redirects.
 */
export const safeRedirectPath = (value: unknown): string | null => {
  if (typeof value !== 'string') return null;
  if (!value.startsWith('/')) return null;
  if (value.startsWith('//') || value.startsWith('/\\')) return null;
  // Control characters (tab, newline…) are stripped by URL parsers and could turn `/\t/host`
  // into `//host`.
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f]/.test(value)) return null;
  return value;
};
