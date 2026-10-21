import { useIsMutating, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { type ApiError, api, type RefreshReport } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

const PORTFOLIO_KEY = ["portfolio"] as const;
/** Queries derived from balances: DeFi positions and the risk report. */
const DERIVED_KEYS = [["positions"], ["risk"]] as const;

function invalidateDerived(queryClient: ReturnType<typeof useQueryClient>) {
  for (const queryKey of DERIVED_KEYS) queryClient.invalidateQueries({ queryKey });
}
/** Balances older than this are refreshed automatically on launch. */
const STALE_MS = 10 * 60_000;

/** Cached portfolio straight from SQLite: instant and offline. */
export function usePortfolio() {
  return useQuery({
    queryKey: PORTFOLIO_KEY,
    queryFn: api.getPortfolio,
    enabled: IS_TAURI,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useHasWallets() {
  const { data } = usePortfolio();
  return (data?.wallets.length ?? 0) > 0;
}

/** Offline classification of what the user typed, for the add dialog. */
export function useDetectAddress(input: string) {
  const trimmed = input.trim();
  return useQuery({
    queryKey: ["detect", trimmed],
    queryFn: () => api.detectAddress(trimmed),
    enabled: IS_TAURI && trimmed.length > 0,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
}

export function useRefreshBalances() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  return useMutation<RefreshReport, ApiError, number | undefined>({
    mutationKey: ["refresh"],
    mutationFn: (walletId) => api.refreshBalances(walletId),
    onSuccess: (report) => {
      queryClient.setQueryData(PORTFOLIO_KEY, report.portfolio);
      invalidateDerived(queryClient);
      if (report.issues.length > 0) {
        toast.warning(t("wallets.syncPartial", { count: report.issues.length }), {
          description: report.issues.map((i) => i.chain ?? i.message).join(", "),
        });
      }
    },
    onError: (error) => {
      toast.error(t("wallets.syncFailed"), { description: error.message });
    },
  });
}

export function useAddWallet() {
  const queryClient = useQueryClient();
  const refresh = useRefreshBalances();
  return useMutation({
    mutationFn: ({ input, label }: { input: string; label?: string }) =>
      api.addWallet(input, label),
    onSuccess: async (wallet) => {
      await queryClient.invalidateQueries({ queryKey: PORTFOLIO_KEY });
      refresh.mutate(wallet.id);
    },
  });
}

export function useRenameWallet() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, label }: { id: number; label: string }) => api.renameWallet(id, label),
    onSettled: () => queryClient.invalidateQueries({ queryKey: PORTFOLIO_KEY }),
  });
}

export function useRemoveWallet() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.removeWallet(id),
    onSettled: () => {
      invalidateDerived(queryClient);
      return queryClient.invalidateQueries({ queryKey: PORTFOLIO_KEY });
    },
  });
}

/** True while any balance refresh is running. */
export function useIsSyncing() {
  return useIsMutating({ mutationKey: ["refresh"] }) > 0;
}

/** Refreshes stale balances once per launch. Mount once, near the root. */
export function useAutoRefresh() {
  const { data } = usePortfolio();
  const refresh = useRefreshBalances();
  const started = useRef(false);

  useEffect(() => {
    if (started.current || !data || data.wallets.length === 0) return;
    started.current = true;
    if (!data.syncedAt || Date.now() - data.syncedAt > STALE_MS) refresh.mutate(undefined);
  }, [data, refresh]);
}
