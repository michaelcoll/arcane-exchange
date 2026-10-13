type ApiOptions = NonNullable<Parameters<typeof $fetch>[1]>;

export const useApi = () => {
  const { getToken } = useAuth();
  const config = useRuntimeConfig();

  const call = async <T>(path: string, options: ApiOptions, authenticated: boolean): Promise<T> => {
    try {
      const headers = authenticated
        ? { ...options.headers, Authorization: `Bearer ${await getToken.value()}` }
        : options.headers;
      // @ts-expect-error -- Nitro types `$fetch<T>` as `TypedInternalResponse<…, T>`, which TS
      // cannot narrow back to the generic `T` (TS2322) although it resolves to `T` for an
      // external URL.
      return await $fetch<T>(`${config.public.apiBase}${path}`, { ...options, headers });
    } catch (e) {
      // Callers only ever see an `ApiError` (ADR 0018), to read with `readApiError`.
      throw readApiError(e);
    }
  };

  const apiCall = <T>(path: string, options: ApiOptions = {}): Promise<T> =>
    call<T>(path, options, true);

  /** For a public endpoint: sends no bearer token, whether the visitor is signed in or not. */
  const publicApiCall = <T>(path: string, options: ApiOptions = {}): Promise<T> =>
    call<T>(path, options, false);

  return { apiCall, publicApiCall };
};
