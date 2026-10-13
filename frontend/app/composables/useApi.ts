export const useApi = () => {
  const { getToken } = useAuth();
  const config = useRuntimeConfig();

  const apiCall = async <T>(
    path: string,
    options: Parameters<typeof $fetch>[1] = {},
  ): Promise<T> => {
    try {
      const token = await getToken.value();
      // @ts-expect-error -- Nitro types `$fetch<T>` as `TypedInternalResponse<…, T>`, which TS
      // cannot narrow back to the generic `T` (TS2322) although it resolves to `T` for an
      // external URL.
      return await $fetch<T>(`${config.public.apiBase}${path}`, {
        ...options,
        headers: {
          ...options.headers,
          Authorization: `Bearer ${token}`,
        },
      });
    } catch (e) {
      // Callers only ever see an `ApiError` (ADR 0018), to read with `readApiError`.
      throw readApiError(e);
    }
  };

  return { apiCall: apiCall };
};
