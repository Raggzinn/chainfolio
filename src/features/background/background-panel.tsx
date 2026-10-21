import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { AppSettings } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";
import { useAppSettings, useUpdateAppSettings } from "./queries";

const INTERVALS = ["0", "5", "15", "30", "60", "240"] as const;
type Interval = (typeof INTERVALS)[number];

/** Settings section: tray, launch at login, background refresh, notifications. */
export function BackgroundPanel() {
  const { t } = useTranslation();
  const { data } = useAppSettings();
  const update = useUpdateAppSettings();
  const disabled = !IS_TAURI || !data;

  const toggle =
    (key: keyof Pick<AppSettings, "closeToTray" | "launchAtLogin" | "notifications">) =>
    (value: boolean) =>
      update.mutate({ [key]: value });

  const intervalLabel = (v: Interval) => {
    const minutes = Number(v);
    if (minutes === 0) return t("background.off");
    return minutes >= 60 ? `${minutes / 60} h` : `${minutes} min`;
  };

  return (
    <Panel title={t("background.title")} subtitle={t("background.subtitle")}>
      <div className="divide-y divide-border">
        <Row label={t("background.interval")} hint={t("background.intervalHint")}>
          <SegmentedControl
            label={t("background.interval")}
            options={INTERVALS}
            labels={Object.fromEntries(INTERVALS.map((v) => [v, intervalLabel(v)]))}
            value={data ? (String(data.syncIntervalMinutes) as Interval) : undefined}
            onChange={(v) => !disabled && update.mutate({ syncIntervalMinutes: Number(v) })}
          />
        </Row>
        <Row
          id="close-to-tray"
          label={t("background.closeToTray")}
          hint={t("background.closeToTrayHint")}
        >
          <Switch
            id="close-to-tray"
            disabled={disabled}
            checked={data?.closeToTray ?? false}
            onCheckedChange={toggle("closeToTray")}
          />
        </Row>
        <Row
          id="launch-at-login"
          label={t("background.launchAtLogin")}
          hint={t("background.launchAtLoginHint")}
        >
          <Switch
            id="launch-at-login"
            disabled={disabled}
            checked={data?.launchAtLogin ?? false}
            onCheckedChange={toggle("launchAtLogin")}
          />
        </Row>
        <Row
          id="notifications"
          label={t("background.notifications")}
          hint={t("background.notificationsHint")}
        >
          <Switch
            id="notifications"
            disabled={disabled}
            checked={data?.notifications ?? false}
            onCheckedChange={toggle("notifications")}
          />
        </Row>
      </div>
    </Panel>
  );
}

function Row({
  id,
  label,
  hint,
  children,
}: {
  id?: string;
  label: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 py-4 first:pt-0 last:pb-0">
      <div className="space-y-1">
        <Label htmlFor={id} className="text-sm">
          {label}
        </Label>
        <p className="max-w-lg text-muted-foreground text-sm">{hint}</p>
      </div>
      {children}
    </div>
  );
}
