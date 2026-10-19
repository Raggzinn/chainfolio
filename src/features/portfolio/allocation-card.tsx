import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cell, Pie, PieChart, ResponsiveContainer } from "recharts";
import { Panel } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { assetValue, CHAINS, DEMO_ASSETS, DEMO_TOTAL } from "@/features/demo/data";
import { useMoney } from "@/hooks/use-money";
import { formatPercent } from "@/lib/format";

type Mode = "asset" | "chain";

interface Slice {
  key: string;
  label: string;
  value: number;
  color: string;
}

function slices(mode: Mode): Slice[] {
  if (mode === "asset") {
    const sorted = [...DEMO_ASSETS].sort((a, b) => assetValue(b) - assetValue(a));
    const top = sorted.slice(0, 5).map((a) => ({
      key: a.symbol,
      label: a.symbol,
      value: assetValue(a),
      color: a.color,
    }));
    const rest = sorted.slice(5).reduce((s, a) => s + assetValue(a), 0);
    return rest > 0
      ? [...top, { key: "other", label: "Other", value: rest, color: "#64748B" }]
      : top;
  }
  const byChain = new Map<string, number>();
  for (const a of DEMO_ASSETS) byChain.set(a.chain, (byChain.get(a.chain) ?? 0) + assetValue(a));
  return [...byChain.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([id, value]) => {
      const chain = CHAINS[id as keyof typeof CHAINS];
      return { key: id, label: chain.name, value, color: chain.color };
    });
}

export function AllocationCard() {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const [mode, setMode] = useState<Mode>("asset");
  const [hovered, setHovered] = useState<string | null>(null);
  const data = useMemo(() => slices(mode), [mode]);
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
            <span className="text-[11px] text-muted-foreground">{active?.label ?? "Total"}</span>
            <span className="tabular font-semibold text-sm">
              {format(active?.value ?? DEMO_TOTAL, { compact: true })}
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
                {formatPercent(s.value / DEMO_TOTAL, i18n.resolvedLanguage ?? "en", {
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
