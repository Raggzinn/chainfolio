import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type ApiError, api, type SecurityStatus } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

export const SECURITY_KEY = ["security"] as const;

export function useSecurityStatus() {
  return useQuery({
    queryKey: SECURITY_KEY,
    queryFn: api.securityStatus,
    enabled: IS_TAURI,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useUnlock() {
  const queryClient = useQueryClient();
  return useMutation<void, ApiError, string>({
    mutationFn: api.unlock,
    onSuccess: () => queryClient.invalidateQueries(),
  });
}

/** Closes the encrypted database and forgets every cached query. */
export function useLock() {
  const queryClient = useQueryClient();
  return useMutation<void, ApiError>({
    mutationFn: api.lock,
    onSuccess: () => {
      queryClient.removeQueries({ predicate: (q) => q.queryKey[0] !== SECURITY_KEY[0] });
      queryClient.setQueryData<SecurityStatus>(SECURITY_KEY, (s) =>
        s ? { ...s, locked: s.encrypted } : s,
      );
    },
  });
}

export function useSetMasterPassword() {
  const queryClient = useQueryClient();
  return useMutation<void, ApiError, { current: string | null; next: string | null }>({
    mutationFn: ({ current, next }) => api.setMasterPassword(current, next),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: SECURITY_KEY }),
  });
}
