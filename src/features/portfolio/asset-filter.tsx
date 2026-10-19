import { Check, ChevronDown, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { TokenIcon } from "@/components/token-icon";
import { Checkbox } from "@/components/ui/checkbox";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { assetValue, DEMO_ASSETS, DEMO_TOTAL } from "@/features/demo/data";
import { formatPercent } from "@/lib/format";
import { cn } from "@/lib/utils";

/** Multi-select of assets. An empty selection means "all assets". */
export function AssetFilter({
  value,
  onChange,
}: {
  value: string[];
  onChange: (symbols: string[]) => void;
}) {
  const { t, i18n } = useTranslation();
  const assets = [...DEMO_ASSETS].sort((a, b) => assetValue(b) - assetValue(a));
  const selected = assets.filter((a) => value.includes(a.symbol));
  const all = selected.length === 0 || selected.length === assets.length;

  const toggle = (symbol: string) => {
    const next = value.includes(symbol) ? value.filter((s) => s !== symbol) : [...value, symbol];
    onChange(next.length === assets.length ? [] : next);
  };

  const label = all
    ? t("filters.allAssets")
    : selected.length === 1
      ? selected[0]?.symbol
      : t("filters.assetsSelected", { count: selected.length });

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          className={cn(
            "flex h-9 items-center gap-2 rounded-xl border bg-card px-3 font-medium text-xs transition-colors hover:bg-accent",
            !all && "border-primary/40 text-primary",
          )}
        >
          {all ? (
            <Layers className="size-4" />
          ) : (
            <span className="-space-x-1.5 flex">
              {selected.slice(0, 3).map((a) => (
                <TokenIcon
                  key={a.symbol}
                  symbol={a.symbol}
                  color={a.color}
                  size={18}
                  className="rounded-full ring-2 ring-card"
                />
              ))}
            </span>
          )}
          {label}
          <ChevronDown className="size-3.5 opacity-60" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-72 p-1.5">
        <button
          type="button"
          onClick={() => onChange([])}
          className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm transition-colors hover:bg-accent"
        >
          <span className="flex size-[18px] items-center justify-center">
            {all && <Check className="size-4 text-primary" />}
          </span>
          <span className="font-medium">{t("filters.allAssets")}</span>
        </button>
        <div className="my-1 h-px bg-border" />
        <ul className="max-h-72 overflow-y-auto">
          {assets.map((a) => {
            const checked = !all && value.includes(a.symbol);
            return (
              <li key={a.symbol}>
                <label
                  htmlFor={`asset-filter-${a.symbol}`}
                  className="flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-accent"
                >
                  <Checkbox
                    id={`asset-filter-${a.symbol}`}
                    checked={checked}
                    onCheckedChange={() => toggle(a.symbol)}
                  />
                  <TokenIcon symbol={a.symbol} color={a.color} size={22} />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate font-medium">{a.symbol}</span>
                    <span className="block truncate text-muted-foreground text-xs">{a.name}</span>
                  </span>
                  <span className="tabular text-muted-foreground text-xs">
                    {formatPercent(assetValue(a) / DEMO_TOTAL, i18n.resolvedLanguage ?? "en", {
                      signed: false,
                    })}
                  </span>
                </label>
              </li>
            );
          })}
        </ul>
      </PopoverContent>
    </Popover>
  );
}
