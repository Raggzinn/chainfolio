import { useId, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { useMoney } from "@/hooks/use-money";

export interface ValuePoint {
  t: number;
  value: number;
}

/** Area chart of portfolio value over time, with a date + value tooltip. */
export function ValueChart({
  data,
  hourly,
  animationKey,
  height = 260,
}: {
  data: ValuePoint[];
  /** Show times in the tooltip (ranges of a few days or less). */
  hourly: boolean;
  /** Changing it replays the draw-in animation. */
  animationKey: string;
  height?: number;
}) {
  const { i18n } = useTranslation();
  const { format } = useMoney();
  const fillId = useId();
  const dateFormat = useMemo(
    () =>
      new Intl.DateTimeFormat(i18n.resolvedLanguage, {
        ...(hourly
          ? { hour: "2-digit", minute: "2-digit", day: "numeric", month: "short" }
          : { day: "numeric", month: "short", year: "numeric" }),
      }),
    [hourly, i18n.resolvedLanguage],
  );

  return (
    <div className="relative" style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart
          key={animationKey}
          data={data}
          margin={{ top: 10, right: 0, bottom: 0, left: 0 }}
        >
          <defs>
            <linearGradient id={fillId} x1="0" y1="0" x2="0" y2="1">
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
              const point = payload?.[0]?.payload as ValuePoint | undefined;
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
            fill={`url(#${fillId})`}
            animationDuration={1100}
            animationEasing="ease-out"
            activeDot={{ r: 5, fill: "var(--primary)", stroke: "var(--card)", strokeWidth: 3 }}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
