import { CheckCircle2, Download, RefreshCw, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { IS_TAURI } from "@/lib/platform";
import { usePreferences } from "@/stores/preferences";
import { useUpdater } from "@/stores/updater";

/** Settings section: current version, manual check and the auto-check toggle. */
export function UpdatesPanel() {
  const { t, i18n } = useTranslation();
  const autoCheck = usePreferences((s) => s.autoCheckUpdates);
  const setAutoCheck = usePreferences((s) => s.setAutoCheckUpdates);
  const { status, update, error, lastCheckedAt, check, install } = useUpdater();
  const time = new Intl.DateTimeFormat(i18n.resolvedLanguage, { timeStyle: "short" });

  const line = (() => {
    if (!IS_TAURI)
      return { icon: TriangleAlert, tone: "text-muted-foreground", text: t("updates.desktopOnly") };
    switch (status) {
      case "checking":
        return {
          icon: RefreshCw,
          tone: "text-muted-foreground",
          text: t("updates.checking"),
          spin: true,
        };
      case "upToDate":
        return { icon: CheckCircle2, tone: "text-success", text: t("updates.upToDate") };
      case "available":
      case "downloading":
      case "ready":
        return {
          icon: Download,
          tone: "text-primary",
          text: t("updates.available", { version: update?.version }),
        };
      case "error":
        return { icon: TriangleAlert, tone: "text-danger", text: t("updates.failed", { error }) };
      default:
        return null;
    }
  })();

  return (
    <Panel title={t("updates.title")} subtitle={t("updates.version", { version: __APP_VERSION__ })}>
      <div className="divide-y divide-border">
        <div className="flex items-center justify-between gap-6 pb-4">
          <div className="min-w-0 space-y-1">
            {line ? (
              <p className={`flex items-center gap-1.5 font-medium text-sm ${line.tone}`}>
                <line.icon className={`size-4 shrink-0 ${line.spin ? "animate-spin" : ""}`} />
                <span className="truncate">{line.text}</span>
              </p>
            ) : (
              <p className="font-medium text-sm">{t("updates.never")}</p>
            )}
            {lastCheckedAt && (
              <p className="text-muted-foreground text-xs">
                {t("updates.lastChecked", { time: time.format(lastCheckedAt) })}
              </p>
            )}
          </div>
          {status === "available" ? (
            <button
              type="button"
              onClick={install}
              className="inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-primary px-3 py-2 font-semibold text-primary-foreground text-sm transition-colors hover:bg-primary/90"
            >
              <Download className="size-4" />
              {t("updates.install")}
            </button>
          ) : (
            <button
              type="button"
              disabled={!IS_TAURI || status === "checking" || status === "downloading"}
              onClick={() => check()}
              className="inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-muted px-3 py-2 font-medium text-sm transition-colors hover:bg-accent disabled:opacity-50"
            >
              <RefreshCw className="size-4" />
              {t("updates.checkNow")}
            </button>
          )}
        </div>
        <div className="flex items-center justify-between gap-6 pt-4">
          <div className="space-y-1">
            <Label htmlFor="auto-updates" className="text-sm">
              {t("updates.auto")}
            </Label>
            <p className="text-muted-foreground text-sm">{t("updates.autoHint")}</p>
          </div>
          <Switch id="auto-updates" checked={autoCheck} onCheckedChange={setAutoCheck} />
        </div>
      </div>
    </Panel>
  );
}
