import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { type ApiError, type AppSettings, api } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

export const APP_SETTINGS_KEY = ["appSettings"] as const;

export function useAppSettings() {
  return useQuery({
    queryKey: APP_SETTINGS_KEY,
    queryFn: api.getAppSettings,
    enabled: IS_TAURI,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

/** Saves a partial change on top of the cached settings, optimistically. */
export function useUpdateAppSettings() {
  const queryClient = useQueryClient();
  return useMutation<AppSettings, ApiError, Partial<AppSettings>, { previous?: AppSettings }>({
    mutationFn: (patch) => {
      const current = queryClient.getQueryData<AppSettings>(APP_SETTINGS_KEY);
      if (!current) throw new Error("settings not loaded");
      return api.updateAppSettings({ ...current, ...patch });
    },
    onMutate: async (patch) => {
      await queryClient.cancelQueries({ queryKey: APP_SETTINGS_KEY });
      const previous = queryClient.getQueryData<AppSettings>(APP_SETTINGS_KEY);
      if (previous) queryClient.setQueryData(APP_SETTINGS_KEY, { ...previous, ...patch });
      return { previous };
    },
    onError: (error, _patch, context) => {
      if (context?.previous) queryClient.setQueryData(APP_SETTINGS_KEY, context.previous);
      toast.error(error.message);
    },
    onSuccess: (settings) => queryClient.setQueryData(APP_SETTINGS_KEY, settings),
  });
}
