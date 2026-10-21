import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cell, Pie, PieChart, ResponsiveContainer } from "recharts";
import { Panel } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { useMoney } from "@/hooks/use-money";
import { CHAINS } from "@/lib/chains";
import { formatPercent } from "@/lib/format";
import { type AssetRow, rowsTotal } from "./model";

type Mode = "asset" | "chain";

interface Slice {
  key: string;
  label: string;
  value: number;
  color: string;
}

function slices(rows: AssetRow[], mode: Mode, other: string): Slice[] {
  if (mode === "asset") {
    const sorted = [...rows].sort((a, b) => b.value - a.value);
    const top = sorted.slice(0, 5).map((a) => ({
      key: a.key,
      label: a.symbol,
      value: a.value,
      color: a.color,
    }));
    const rest = rowsTotal(sorted.slice(5));
    return rest > 0 ? [...top, { key: "other", label: other, value: rest, color: "#64748B" }] : top;
  }
  const byChain = new Map<AssetRow["chain"], number>();
  for (const a of rows) byChain.set(a.chain, (byChain.get(a.chain) ?? 0) + a.value);
  return [...byChain.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([id, value]) => ({ key: id, label: CHAINS[id].name, value, color: CHAINS[id].color }));
}

export function AllocationCard({ rows }: { rows: AssetRow[] }) {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const [mode, setMode] = useState<Mode>("asset");
  const [hovered, setHovered] = useState<string | null>(null);
  const other = t("dashboard.other");
  const data = useMemo(() => slices(rows, mode, other), [rows, mode, other]);
  const total = rowsTotal(rows);
  const active = data.find((s) => s.key === hovered);

  return (
    <Panel
      title={t("dashboard.allocation")}
      action={
        <SegmentedControl
          label={t("dashboard.allocation")}
          options={["asset", "chain"] as const}
          labels={{ asset: t("dashboard.byAsset"), chain: t("dashboard.byChain") }}
          value={mode}
          onChange={setMode}
        />
      }
    >
      <div className="flex items-center gap-6">
        <div className="relative size-[168px] shrink-0">
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                key={mode}
                data={data}
                dataKey="value"
                nameKey="label"
                innerRadius={58}
                outerRadius={80}
                paddingAngle={3}
                cornerRadius={6}
                stroke="none"
                animationDuration={900}
                animationEasing="ease-out"
                onMouseEnter={(_, i) => setHovered(data[i]?.key ?? null)}
                onMouseLeave={() => setHovered(null)}
              >
                {data.map((s) => (
                  <Cell
                    key={s.key}
                    fill={s.color}
                    opacity={hovered && hovered !== s.key ? 0.35 : 1}
                    style={{ transition: "opacity 200ms", outline: "none" }}
                  />
                ))}
              </Pie>
            </PieChart>
          </ResponsiveContainer>
          <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
            <span className="text-[11px] text-muted-foreground">
              {active?.label ?? t("dashboard.total")}
            </span>
            <span className="tabular font-semibold text-sm">
              {format(active?.value ?? total, { compact: true })}
            </span>
          </div>
        </div>

        <ul className="min-w-0 flex-1 space-y-2">
          {data.map((s) => (
            <li
              key={s.key}
              onMouseEnter={() => setHovered(s.key)}
              onMouseLeave={() => setHovered(null)}
              className="flex items-center gap-2.5 rounded-lg px-2 py-1 text-sm transition-colors hover:bg-accent"
            >
              <span className="size-2.5 shrink-0 rounded-full" style={{ background: s.color }} />
              <span className="truncate">{s.label}</span>
              <span className="tabular ml-auto font-medium text-muted-foreground">
                {formatPercent(total ? s.value / total : 0, i18n.resolvedLanguage ?? "en", {
                  signed: false,
                })}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </Panel>
  );
}
