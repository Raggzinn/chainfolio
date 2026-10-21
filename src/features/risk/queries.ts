import { useIsMutating, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { type ApiError, api, type RiskScan } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

export const RISK_KEY = ["risk"] as const;
/** Approvals and token verdicts older than this are rescanned automatically. */
const SCAN_STALE_MS = 6 * 60 * 60_000;

/** Risk report computed from local data: instant and offline. */
export function useRiskReport() {
  return useQuery({
    queryKey: RISK_KEY,
    queryFn: api.getRiskReport,
    enabled: IS_TAURI,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useScanRisk() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  return useMutation<RiskScan, ApiError>({
    mutationKey: ["risk-scan"],
    mutationFn: api.scanRisk,
    onSuccess: (scan) => {
      queryClient.setQueryData(RISK_KEY, scan.report);
      // Newly flagged spam leaves the portfolio totals.
      queryClient.invalidateQueries({ queryKey: ["portfolio"] });
      if (scan.issues.length > 0) {
        toast.warning(t("risk.scanPartial", { count: scan.issues.length }), {
          description: scan.issues.map((i) => i.message).join("\n"),
        });
      }
    },
    onError: (error) => {
      toast.error(t("risk.scanFailed"), { description: error.message });
    },
  });
}

export function useIsScanning() {
  return useIsMutating({ mutationKey: ["risk-scan"] }) > 0;
}

let autoScanned = false;

/** Scans once per launch when the last scan is stale and wallets exist. */
export function useAutoScan(hasWallets: boolean) {
  const { data } = useRiskReport();
  const scan = useScanRisk();
  const started = useRef(false);

  useEffect(() => {
    if (autoScanned || started.current || !data || !hasWallets) return;
    started.current = true;
    autoScanned = true;
    if (!data.scannedAt || Date.now() - data.scannedAt > SCAN_STALE_MS) scan.mutate();
  }, [data, hasWallets, scan]);
}
