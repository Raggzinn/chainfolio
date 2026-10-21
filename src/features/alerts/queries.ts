import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { type AlertCondition, type ApiError, api } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

export const ALERT_RULES_KEY = ["alerts", "rules"] as const;
export const ALERT_EVENTS_KEY = ["alerts", "events"] as const;

export function useAlertRules() {
  return useQuery({ queryKey: ALERT_RULES_KEY, queryFn: api.listAlertRules, enabled: IS_TAURI });
}

export function useAlertEvents() {
  return useQuery({
    queryKey: ALERT_EVENTS_KEY,
    queryFn: () => api.listAlertEvents(50),
    enabled: IS_TAURI,
  });
}

export function useWatchableAssets(enabled: boolean) {
  return useQuery({
    queryKey: ["alerts", "assets"],
    queryFn: api.alertAssets,
    enabled: IS_TAURI && enabled,
  });
}

function useInvalidateAlerts() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries({ queryKey: ["alerts"] });
}

function onError(error: ApiError) {
  toast.error(error.message);
}

export function useSaveAlertRule() {
  const invalidate = useInvalidateAlerts();
  return useMutation<unknown, ApiError, { id?: number; condition: AlertCondition }>({
    mutationFn: ({ id, condition }) =>
      id === undefined ? api.createAlertRule(condition) : api.updateAlertRule(id, condition),
    onSettled: invalidate,
  });
}

export function useToggleAlertRule() {
  const invalidate = useInvalidateAlerts();
  return useMutation<void, ApiError, { id: number; enabled: boolean }>({
    mutationFn: ({ id, enabled }) => api.setAlertRuleEnabled(id, enabled),
    onError,
    onSettled: invalidate,
  });
}

export function useDeleteAlertRule() {
  const invalidate = useInvalidateAlerts();
  return useMutation<void, ApiError, number>({
    mutationFn: api.deleteAlertRule,
    onError,
    onSettled: invalidate,
  });
}

export function useClearAlertEvents() {
  const invalidate = useInvalidateAlerts();
  return useMutation<void, ApiError>({
    mutationFn: api.clearAlertEvents,
    onError,
    onSettled: invalidate,
  });
}
