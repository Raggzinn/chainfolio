import { Link } from "@tanstack/react-router";
import {
  ArrowDownUp,
  BellRing,
  Pencil,
  Plus,
  ShieldAlert,
  Tag,
  Trash2,
  TrendingUp,
} from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";
import { Panel } from "@/components/panel";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { type Describer, describeCondition, describeEvent } from "@/features/alerts/describe";
import {
  useAlertEvents,
  useAlertRules,
  useClearAlertEvents,
  useDeleteAlertRule,
  useToggleAlertRule,
} from "@/features/alerts/queries";
import { RuleDialog } from "@/features/alerts/rule-dialog";
import { useAppSettings } from "@/features/background/queries";
import { usePortfolio } from "@/features/wallets/queries";
import type { AlertKind, AlertRule } from "@/lib/api";
import { formatRelative } from "@/lib/format";
import { IS_TAURI } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { usePreferences } from "@/stores/preferences";

const KIND_ICON: Record<AlertKind, typeof Tag> = {
  priceAbove: Tag,
  priceBelow: Tag,
  portfolioChange: TrendingUp,
  balanceChange: ArrowDownUp,
  depeg: ShieldAlert,
};

function useDescriber(): Describer {
  const { t, i18n } = useTranslation();
  const { data: portfolio } = usePortfolio();
  const hidden = usePreferences((s) => s.privacyMode);
  const wallet = useCallback(
    (id: number | null) =>
      id === null
        ? t("alerts.anyWallet")
        : (portfolio?.wallets.find((w) => w.id === id)?.label ?? t("alerts.removedWallet")),
    [portfolio, t],
  );
  return { t, locale: i18n.resolvedLanguage ?? "en", wallet, hidden };
}

export function AlertsPage() {
  const { t } = useTranslation();
  const [editing, setEditing] = useState<AlertRule | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const rules = useAlertRules();
  const { data: settings } = useAppSettings();

  const openDialog = (rule: AlertRule | null) => {
    setEditing(rule);
    setDialogOpen(true);
  };

  const newButton = (
    <Button onClick={() => openDialog(null)} disabled={!IS_TAURI} className="rounded-xl">
      <Plus />
      {t("alerts.new")}
    </Button>
  );

  const interval = settings?.syncIntervalMinutes ?? 0;

  return (
    <>
      <PageHeader
        title={t("pages.alerts.title")}
        description={t("pages.alerts.description")}
        actions={newButton}
      />

      {IS_TAURI && rules.isPending ? (
        <Skeleton className="h-40 rounded-2xl" />
      ) : rules.data && rules.data.length > 0 ? (
        <>
          <p className="text-muted-foreground text-sm">
            {interval > 0 ? t("alerts.schedule", { minutes: interval }) : t("alerts.scheduleOff")}{" "}
            <Link to="/settings" className="text-primary hover:underline">
              {t("alerts.changeSchedule")}
            </Link>
          </p>
          <RuleList rules={rules.data} onEdit={openDialog} />
          <HistoryPanel />
        </>
      ) : (
        <EmptyState
          icon={BellRing}
          title={t("alerts.emptyTitle")}
          description={IS_TAURI ? t("alerts.emptyHint") : t("wallets.desktopOnly")}
          action={newButton}
        />
      )}

      <RuleDialog open={dialogOpen} rule={editing} onOpenChange={setDialogOpen} />
    </>
  );
}

function RuleList({ rules, onEdit }: { rules: AlertRule[]; onEdit: (rule: AlertRule) => void }) {
  const { t, i18n } = useTranslation();
  const describer = useDescriber();
  const toggle = useToggleAlertRule();
  const remove = useDeleteAlertRule();
  const locale = i18n.resolvedLanguage ?? "en";

  return (
    <Panel title={t("alerts.rules")} subtitle={t("alerts.rulesCount", { count: rules.length })}>
      <ul className="-mx-5 -mb-5 divide-y divide-border">
        {rules.map((rule) => {
          const Icon = KIND_ICON[rule.condition.kind];
          const holding = rule.enabled && !rule.armed && rule.condition.kind !== "balanceChange";
          return (
            <li key={rule.id} className="flex items-center gap-4 px-5 py-3.5">
              <span
                className={cn(
                  "flex size-9 shrink-0 items-center justify-center rounded-lg",
                  rule.enabled ? "bg-primary/12 text-primary" : "bg-muted text-muted-foreground",
                )}
              >
                <Icon className="size-4" />
              </span>
              <div className="min-w-0 flex-1">
                <p
                  className={cn(
                    "truncate font-medium text-sm",
                    !rule.enabled && "text-muted-foreground",
                  )}
                >
                  {describeCondition(rule.condition, describer)}
                </p>
                <p className="text-muted-foreground text-xs">
                  {rule.lastTriggeredAt
                    ? t("alerts.lastFired", { time: formatRelative(rule.lastTriggeredAt, locale) })
                    : t("alerts.neverFired")}
                  {holding && ` · ${t("alerts.waitingReset")}`}
                </p>
              </div>
              <Switch
                checked={rule.enabled}
                aria-label={t("alerts.enabled")}
                onCheckedChange={(enabled) => toggle.mutate({ id: rule.id, enabled })}
              />
              <IconAction label={t("alerts.edit")} onClick={() => onEdit(rule)}>
                <Pencil />
              </IconAction>
              <IconAction label={t("alerts.delete")} onClick={() => remove.mutate(rule.id)} danger>
                <Trash2 />
              </IconAction>
            </li>
          );
        })}
      </ul>
    </Panel>
  );
}

function HistoryPanel() {
  const { t, i18n } = useTranslation();
  const describer = useDescriber();
  const events = useAlertEvents();
  const clear = useClearAlertEvents();
  const locale = i18n.resolvedLanguage ?? "en";
  const list = events.data ?? [];

  return (
    <Panel
      title={t("alerts.history")}
      subtitle={t("alerts.historyHint")}
      action={
        list.length > 0 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => clear.mutate()}
            className="rounded-lg text-muted-foreground"
          >
            {t("alerts.clearHistory")}
          </Button>
        )
      }
    >
      {list.length === 0 ? (
        <p className="py-6 text-center text-muted-foreground text-sm">{t("alerts.noHistory")}</p>
      ) : (
        <ul className="space-y-1">
          {list.map((event) => {
            const Icon = KIND_ICON[event.condition.kind];
            return (
              <li key={event.id} className="flex items-center gap-3 rounded-lg px-1 py-2">
                <Icon className="size-4 shrink-0 text-muted-foreground" />
                <span className="min-w-0 flex-1 truncate text-sm">
                  {describeEvent(event, describer)}
                </span>
                <time
                  dateTime={new Date(event.triggeredAt).toISOString()}
                  className="shrink-0 text-muted-foreground text-xs tabular-nums"
                >
                  {formatRelative(event.triggeredAt, locale)}
                </time>
              </li>
            );
          })}
        </ul>
      )}
    </Panel>
  );
}

function IconAction({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          aria-label={label}
          onClick={onClick}
          className={cn("size-8 rounded-lg", danger && "hover:text-danger")}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
