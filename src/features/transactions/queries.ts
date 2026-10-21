import {
  keepPreviousData,
  useInfiniteQuery,
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { type ApiError, api, type HistoryReport, type TxKind, type TxQuery } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

const PAGE = 50;
export const TRANSACTIONS_KEY = ["transactions"] as const;
export const HISTORY_KEY = ["portfolio-history"] as const;

/** Pulls new transactions in the background; never blocks the UI. */
export function useSyncHistory() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  return useMutation<HistoryReport, ApiError, number | undefined>({
    mutationKey: ["history-sync"],
    mutationFn: (walletId) => api.syncHistory(walletId),
    onSuccess: (report) => {
      queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY });
      queryClient.invalidateQueries({ queryKey: HISTORY_KEY });
      if (report.issues.length > 0) {
        toast.warning(t("transactions.syncPartial", { count: report.issues.length }), {
          description: report.issues.map((i) => i.chain).join(", "),
        });
      }
    },
  });
}

export function useIsSyncingHistory() {
  return useIsMutating({ mutationKey: ["history-sync"] }) > 0;
}

/** Paginated transaction list for the given filters. */
export function useTransactions(query: TxQuery) {
  return useInfiniteQuery({
    queryKey: [...TRANSACTIONS_KEY, "list", query],
    queryFn: ({ pageParam }) => api.listTransactions({ ...query, limit: PAGE, offset: pageParam }),
    initialPageParam: 0,
    getNextPageParam: (last, pages) => {
      const loaded = pages.reduce((n, p) => n + p.items.length, 0);
      return loaded < last.total ? loaded : undefined;
    },
    enabled: IS_TAURI,
    placeholderData: keepPreviousData,
  });
}

export function useRecentTransactions(limit = 5) {
  return useQuery({
    queryKey: [...TRANSACTIONS_KEY, "recent", limit],
    queryFn: () => api.listTransactions({ limit }),
    enabled: IS_TAURI,
  });
}

export function useUpdateTransaction() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      txId,
      walletId,
      kind,
      note,
    }: {
      txId: string;
      walletId: number;
      kind: TxKind | null;
      note: string | null;
    }) => api.updateTransaction(txId, walletId, kind, note),
    onSettled: () => queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY }),
  });
}

/** Builds the CSV for `query` and hands it to the browser as a download. */
export async function downloadTransactionsCsv(query: TxQuery) {
  const csv = await api.exportTransactionsCsv(query);
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `chainfolio-transactions-${new Date().toISOString().slice(0, 10)}.csv`;
  link.click();
  URL.revokeObjectURL(url);
}

/** Portfolio value between two dates, rounded to the minute so it caches. */
export function usePortfolioHistory(from: number, to: number, assetIds?: string[]) {
  const minute = 60_000;
  const range = [Math.floor(from / minute) * minute, Math.floor(to / minute) * minute] as const;
  const assets = assetIds?.length ? [...assetIds].sort() : null;
  return useQuery({
    queryKey: [...HISTORY_KEY, range, assets],
    queryFn: () => api.portfolioHistory(range[0], range[1], assets),
    enabled: IS_TAURI,
    placeholderData: keepPreviousData,
    staleTime: 5 * minute,
  });
}

export function useApiKeys() {
  return useQuery({ queryKey: ["api-keys"], queryFn: api.getApiKeys, enabled: IS_TAURI });
}

export function useSetApiKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ provider, key }: { provider: "etherscan"; key: string | null }) =>
      api.setApiKey(provider, key),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["api-keys"] }),
  });
}
