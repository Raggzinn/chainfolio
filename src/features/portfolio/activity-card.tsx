import { ArrowDownLeft, ArrowLeftRight, ArrowUpRight, Layers, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { type ActivityKind, CHAINS, DEMO_ACTIVITY } from "@/features/demo/data";
import { useMoney } from "@/hooks/use-money";
import { cn } from "@/lib/utils";

const KIND: Record<ActivityKind, { icon: LucideIcon; tone: string }> = {
  received: { icon: ArrowDownLeft, tone: "bg-success/12 text-success" },
  sent: { icon: ArrowUpRight, tone: "bg-danger/12 text-danger" },
  swap: { icon: ArrowLeftRight, tone: "bg-primary/12 text-primary" },
  staked: { icon: Layers, tone: "bg-chart-4/15 text-chart-4" },
};

export function ActivityCard() {
  const { t } = useTranslation();
  const { format, hidden } = useMoney();

  const ago = (minutes: number) =>
    minutes < 60
      ? t("dashboard.minutesAgo", { count: minutes })
      : minutes < 1440
        ? t("dashboard.hoursAgo", { count: Math.round(minutes / 60) })
        : t("dashboard.daysAgo", { count: Math.round(minutes / 1440) });

  return (
    <Panel title={t("dashboard.recentActivity")}>
      <ol className="-mx-1 space-y-1">
        {DEMO_ACTIVITY.map((item) => {
          const { icon: Icon, tone } = KIND[item.kind];
          return (
            <li
              key={`${item.kind}-${item.minutesAgo}`}
              className="relative flex items-center gap-3 rounded-xl px-1 py-2 transition-colors hover:bg-accent"
            >
              <span
                className={cn("flex size-9 shrink-0 items-center justify-center rounded-xl", tone)}
              >
                <Icon className="size-4" strokeWidth={2.2} />
              </span>
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium text-sm">
                  {t(`dashboard.activity.${item.kind}`)} {item.label}
                </p>
                <p className="text-muted-foreground text-xs">
                  {CHAINS[item.chain].name} · {ago(item.minutesAgo)}
                </p>
              </div>
              <div className="text-right">
                <p className="tabular font-medium text-sm">{hidden ? "•••" : item.amount}</p>
                <p className="tabular text-muted-foreground text-xs">
                  {format(Math.abs(item.value))}
                </p>
              </div>
            </li>
          );
        })}
      </ol>
    </Panel>
  );
}
