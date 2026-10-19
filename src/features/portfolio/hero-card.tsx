import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { ChangePill } from "@/components/change-pill";
import { AnimatedNumber } from "@/components/effects/animated-number";
import { SegmentedControl } from "@/components/segmented-control";
import {
  DEMO_ASSETS,
  DEMO_CHANGE_24H,
  DEMO_COST_BASIS,
  DEMO_TOTAL,
  demoHistory,
  RANGES,
  type Range,
} from "@/features/demo/data";
import { useMoney } from "@/hooks/use-money";
import { cn } from "@/lib/utils";

export function HeroCard() {
  const { t, i18n } = useTranslation();
  const { format, hidden } = useMoney();
  const [range, setRange] = useState<Range>("1M");
  const data = useMemo(() => demoHistory(range), [range]);

  const first = data[0]?.value ?? DEMO_TOTAL;
  const rangeChange = DEMO_TOTAL / first - 1;
  const todayDelta = DEMO_TOTAL * (DEMO_CHANGE_24H / (1 + DEMO_CHANGE_24H));
  const pnl = DEMO_TOTAL - DEMO_COST_BASIS;
  const chains = new Set(DEMO_ASSETS.map((a) => a.chain)).size;

  const dateFormat = useMemo(
    () =>
      new Intl.DateTimeFormat(i18n.resolvedLanguage, {
        ...(range === "1D" || range === "1W"
          ? { hour: "2-digit", minute: "2-digit", day: "numeric", month: "short" }
          : { day: "numeric", month: "short", year: "numeric" }),
      }),
    [range, i18n.resolvedLanguage],
  );

  return (
    <section className="surface relative overflow-hidden rounded-3xl">
      <div className="relative flex flex-wrap items-start justify-between gap-6 p-6 pb-0">
        <div className="space-y-3">
          <p className="font-medium text-muted-foreground text-sm">{t("dashboard.totalValue")}</p>
          <AnimatedNumber
            value={DEMO_TOTAL}
            format={(n) => format(n, { ignorePrivacy: true })}
            hidden={hidden}
            className="tabular block font-semibold text-[52px] leading-none tracking-[-0.035em]"
          />
          <div className="flex items-center gap-2.5 text-sm">
            <ChangePill value={rangeChange} size="md" />
            <span className="tabular text-muted-foreground">
              <span className={cn("font-medium", todayDelta >= 0 ? "text-success" : "text-danger")}>
                {todayDelta >= 0 ? "+" : ""}
                {format(todayDelta)}
              </span>{" "}
              {t("dashboard.today")}
            </span>
          </div>
        </div>
        <SegmentedControl label="Range" options={RANGES} value={range} onChange={setRange} />
      </div>

      <div className="relative mt-4 h-[260px]">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart key={range} data={data} margin={{ top: 10, right: 0, bottom: 0, left: 0 }}>
            <defs>
              <linearGradient id="hero-fill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#1E6BFF" stopOpacity={0.22} />
                <stop offset="60%" stopColor="#1E6BFF" stopOpacity={0.08} />
                <stop offset="100%" stopColor="#1E6BFF" stopOpacity={0} />
              </linearGradient>
            </defs>
            <XAxis dataKey="t" hide />
            <YAxis hide domain={["dataMin * 0.985", "dataMax * 1.01"]} />
            <Tooltip
              cursor={{
                stroke: "var(--muted-foreground)",
                strokeOpacity: 0.35,
                strokeDasharray: "4 4",
              }}
              content={({ active, payload }) => {
                const point = payload?.[0]?.payload as { t: number; value: number } | undefined;
                if (!active || !point) return null;
                return (
                  <div className="surface-raised rounded-xl px-3 py-2 text-xs">
                    <p className="text-muted-foreground">{dateFormat.format(point.t)}</p>
                    <p className="tabular mt-0.5 font-semibold text-sm">{format(point.value)}</p>
                  </div>
                );
              }}
            />
            <Area
              type="monotone"
              dataKey="value"
              stroke="var(--primary)"
              strokeWidth={2.5}
              fill="url(#hero-fill)"
              animationDuration={1100}
              animationEasing="ease-out"
              activeDot={{ r: 5, fill: "var(--primary)", stroke: "var(--card)", strokeWidth: 3 }}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      <div className="relative grid grid-cols-3 divide-x divide-border border-t">
        <Stat label={t("dashboard.allTime")}>
          <span className={pnl >= 0 ? "text-success" : "text-danger"}>
            {pnl >= 0 ? "+" : ""}
            {format(pnl)}
          </span>
        </Stat>
        <Stat label={t("dashboard.costBasis")}>{format(DEMO_COST_BASIS)}</Stat>
        <Stat label={t("dashboard.assetsCount", { count: DEMO_ASSETS.length })}>
          <span className="text-muted-foreground text-sm">
            {t("dashboard.chainsCount", { count: chains })}
          </span>
        </Stat>
      </div>
    </section>
  );
}

function Stat({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1 px-6 py-4">
      <p className="text-muted-foreground text-xs">{label}</p>
      <p className="tabular font-semibold text-lg tracking-tight">{children}</p>
    </div>
  );
}
