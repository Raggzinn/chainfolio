import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { PORTFOLIO_KEY } from "@/features/wallets/queries";
import { api, type RefreshReport } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";
import { usePreferences } from "@/stores/preferences";
import { useAppSettings, useUpdateAppSettings } from "./queries";

/**
 * Bridges what the backend does on its own (background sync, tray menu) with
 * the UI, and keeps the tray in step with the UI. Mount once, near the root.
 */
export function useBackendEvents() {
  const queryClient = useQueryClient();
  const privacyMode = usePreferences((s) => s.privacyMode);
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { data: settings } = useAppSettings();
  const { mutate: updateSettings } = useUpdateAppSettings();

  useEffect(() => {
    if (!IS_TAURI) return;
    const unlisten = [
      listen<RefreshReport>("sync://completed", ({ payload }) => {
        queryClient.setQueryData(PORTFOLIO_KEY, payload.portfolio);
      }),
      listen<unknown[]>("alerts://triggered", ({ payload }) => {
        queryClient.invalidateQueries({ queryKey: ["alerts"] });
        toast.info(t("alerts.fired", { count: payload.length }), {
          action: { label: t("alerts.view"), onClick: () => navigate({ to: "/alerts" }) },
        });
      }),
      listen<boolean>("tray://privacy", ({ payload }) => {
        usePreferences.setState({ privacyMode: payload });
      }),
    ];
    return () => {
      for (const p of unlisten) p.then((off) => off());
    };
  }, [queryClient, t, navigate]);

  useEffect(() => {
    if (IS_TAURI) api.setTrayPrivacy(privacyMode).catch(() => {});
  }, [privacyMode]);

  // Tray labels and notifications follow the UI language.
  const language = i18n.resolvedLanguage === "pt-BR" ? "pt-BR" : "en";
  useEffect(() => {
    if (settings && settings.locale !== language) updateSettings({ locale: language });
  }, [settings, language, updateSettings]);
}
