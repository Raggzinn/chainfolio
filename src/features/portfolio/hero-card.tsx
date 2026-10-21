import { useMemo, useState } from "react";
import type { DateRange } from "react-day-picker";
import { useTranslation } from "react-i18next";
import { ChangePill } from "@/components/change-pill";
import { AnimatedNumber } from "@/components/effects/animated-number";
import { SegmentedControl } from "@/components/segmented-control";
import {
  assetValue,
  DEMO_ASSETS,
  demoPortfolioHistory,
  demoRangeHistory,
  RANGES,
  type Range,
} from "@/features/demo/data";
import { useMoney } from "@/hooks/use-money";
import { cn } from "@/lib/utils";
import { AssetFilter } from "./asset-filter";
import { DateRangePicker } from "./date-range-picker";
import { demoAssetRows } from "./model";
import { ValueChart } from "./value-chart";

const DEMO_ROWS = demoAssetRows();

const HOURLY_RANGES: Range[] = ["1D", "1W"];

export function HeroCard() {
  const { t } = useTranslation();
  const { format, hidden } = useMoney();
  const [range, setRange] = useState<Range>("1M");
  const [customRange, setCustomRange] = useState<DateRange>();
  const [useCustom, setUseCustom] = useState(false);
  const [symbols, setSymbols] = useState<string[]>([]);

  const custom = useCustom && customRange?.from && customRange.to ? customRange : undefined;
  const data = useMemo(
    () =>
      custom?.from && custom.to
        ? demoRangeHistory(custom.from, custom.to, symbols)
        : demoPortfolioHistory(range, symbols),
    [custom, range, symbols],
  );

  const selected = symbols.length
    ? DEMO_ASSETS.filter((a) => symbols.includes(a.symbol))
    : DEMO_ASSETS;
  const first = data[0]?.value ?? 0;
  const last = data.at(-1)?.value ?? 0;
  const rangeChange = first ? last / first - 1 : 0;
  const rangeDelta = last - first;
  const costBasis = selected.reduce((sum, a) => sum + a.costBasis, 0);
  const current = selected.reduce((sum, a) => sum + assetValue(a), 0);
  const pnl = current - costBasis;
  const chains = new Set(selected.map((a) => a.chain)).size;

  const periodLabel = custom ? t("filters.inPeriod") : t(`filters.ranges.${range}`);

  return (
    <section data-tour="portfolio" className="surface relative overflow-hidden rounded-3xl">
      <div className="relative flex flex-wrap items-start justify-between gap-6 p-6 pb-0">
        <div className="space-y-3">
          <p className="font-medium text-muted-foreground text-sm">
            {symbols.length ? t("filters.selectedValue") : t("dashboard.totalValue")}
          </p>
          <AnimatedNumber
            value={last}
            format={(n) => format(n, { ignorePrivacy: true })}
            hidden={hidden}
            className="tabular block font-semibold text-[52px] leading-none tracking-[-0.035em]"
          />
          <div className="flex items-center gap-2.5 text-sm">
            <ChangePill value={rangeChange} size="md" />
            <span className="tabular text-muted-foreground">
              <span className={cn("font-medium", rangeDelta >= 0 ? "text-success" : "text-danger")}>
                {rangeDelta >= 0 ? "+" : ""}
                {format(rangeDelta)}
              </span>{" "}
              {periodLabel}
            </span>
          </div>
        </div>

        <div data-tour="filters" className="flex flex-col items-end gap-2">
          <SegmentedControl
            label={t("filters.period")}
            options={RANGES}
            value={custom ? undefined : range}
            onChange={(r) => {
              setUseCustom(false);
              setRange(r);
            }}
          />
          <div className="flex items-center gap-2">
            <AssetFilter rows={DEMO_ROWS} value={symbols} onChange={setSymbols} />
            <DateRangePicker
              value={customRange}
              active={Boolean(custom)}
              onChange={(r) => {
                setCustomRange(r);
                setUseCustom(true);
              }}
            />
          </div>
        </div>
      </div>

      <div className="mt-4">
        <ValueChart
          data={data.map((p) => ({ t: p.t, value: p.value }))}
          hourly={!custom && HOURLY_RANGES.includes(range)}
          animationKey={`${custom?.from?.getTime() ?? range}-${symbols.join()}`}
        />
      </div>

      <div className="relative grid grid-cols-3 divide-x divide-border border-t">
        <Stat label={t("dashboard.allTime")}>
          <span className={pnl >= 0 ? "text-success" : "text-danger"}>
            {pnl >= 0 ? "+" : ""}
            {format(pnl)}
          </span>
        </Stat>
        <Stat label={t("dashboard.costBasis")}>{format(costBasis)}</Stat>
        <Stat label={t("dashboard.assetsCount", { count: selected.length })}>
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
