import { Check, ChevronDown, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { TokenIcon } from "@/components/token-icon";
import { Checkbox } from "@/components/ui/checkbox";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import type { ChainId } from "@/lib/api";
import { CHAINS } from "@/lib/chains";
import { formatPercent } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { AssetRow } from "./model";

/** Multi-select of assets by row key. An empty selection means "all assets". */
export function AssetFilter({
  rows,
  value,
  onChange,
}: {
  /** Selectable assets, largest first. */
  rows: AssetRow[];
  value: string[];
  onChange: (keys: string[]) => void;
}) {
  const { t, i18n } = useTranslation();
  const total = rows.reduce((sum, r) => sum + r.value, 0);
  const selected = rows.filter((r) => value.includes(r.key));
  const all = selected.length === 0 || selected.length === rows.length;
  const showChain = new Set(rows.map((r) => r.chain)).size > 1;

  const toggle = (key: string) => {
    const next = value.includes(key) ? value.filter((k) => k !== key) : [...value, key];
    onChange(next.length === rows.length ? [] : next);
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
            <span className="flex -space-x-1.5">
              {selected.slice(0, 3).map((r) => (
                <TokenIcon
                  key={r.key}
                  symbol={r.symbol}
                  color={r.color}
                  src={r.logoUrl}
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
          {rows.map((r) => {
            const checked = !all && value.includes(r.key);
            const id = `asset-filter-${r.key.replace(/[^a-z0-9]/gi, "-")}`;
            return (
              <li key={r.key}>
                <label
                  htmlFor={id}
                  className="flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-accent"
                >
                  <Checkbox id={id} checked={checked} onCheckedChange={() => toggle(r.key)} />
                  <TokenIcon symbol={r.symbol} color={r.color} src={r.logoUrl} size={22} />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate font-medium">{r.symbol}</span>
                    <span className="block truncate text-muted-foreground text-xs">
                      {showChain ? CHAINS[r.chain as ChainId].name : r.name}
                    </span>
                  </span>
                  <span className="tabular text-muted-foreground text-xs">
                    {formatPercent(total ? r.value / total : 0, i18n.resolvedLanguage ?? "en", {
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
