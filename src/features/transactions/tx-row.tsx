import { StickyNote } from "lucide-react";
import { useTranslation } from "react-i18next";
import { TokenIcon } from "@/components/token-icon";
import { useMoney } from "@/hooks/use-money";
import type { TxItem } from "@/lib/api";
import { CHAINS, tokenColor } from "@/lib/chains";
import { cn } from "@/lib/utils";
import { headlineValue, KIND_STYLE, legSymbols } from "./tx-kind";

export function useTxTitle() {
  const { t } = useTranslation();
  return (tx: TxItem) => {
    const { in: received, out: sent } = legSymbols(tx);
    const kind = t(`transactions.kinds.${tx.kind}`);
    if (tx.kind === "swap" && received && sent) return `${kind} ${sent} → ${received}`;
    const symbols = received || sent;
    return symbols ? `${kind} ${symbols}` : kind;
  };
}

/** One transaction in the list: what happened, to which wallet, and the amounts. */
export function TxRow({ tx, onOpen }: { tx: TxItem; onOpen: (tx: TxItem) => void }) {
  const { i18n } = useTranslation();
  const { format, hidden } = useMoney();
  const title = useTxTitle();
  const { icon: Icon, tone } = KIND_STYLE[tx.kind];
  const time = new Intl.DateTimeFormat(i18n.resolvedLanguage, { timeStyle: "short" });
  const amount = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumFractionDigits: 6 });
  const value = headlineValue(tx);
  const lead = tx.legs.find((l) => Number(l.amount) > 0) ?? tx.legs[0];

  return (
    <button
      type="button"
      onClick={() => onOpen(tx)}
      className="group flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors hover:bg-accent"
    >
      <span className="relative shrink-0">
        <span className={cn("flex size-10 items-center justify-center rounded-xl", tone)}>
          <Icon className="size-[18px]" strokeWidth={2.2} />
        </span>
        {lead && (
          <TokenIcon
            symbol={lead.asset.symbol}
            color={tokenColor(lead.asset.symbol, lead.asset.chain)}
            src={lead.asset.logoUrl}
            size={18}
            className="absolute -right-1.5 -bottom-1.5 rounded-full ring-2 ring-card"
          />
        )}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-1.5">
          <span className="truncate font-medium text-sm">{title(tx)}</span>
          {tx.note && <StickyNote className="size-3.5 shrink-0 text-muted-foreground" />}
        </span>
        <span className="flex items-center gap-1.5 truncate text-muted-foreground text-xs">
          <span
            className="size-1.5 shrink-0 rounded-full"
            style={{ background: CHAINS[tx.chain].color }}
          />
          {CHAINS[tx.chain].name} · {tx.walletLabel} · {time.format(tx.timestamp)}
        </span>
      </span>
      <span className="shrink-0 text-right">
        <span className="block space-y-0.5">
          {tx.legs.slice(0, 2).map((leg) => {
            const n = Number(leg.amount);
            return (
              <span
                key={leg.asset.id}
                className={cn(
                  "tabular block font-medium text-sm",
                  n > 0 ? "text-success" : "text-foreground",
                )}
              >
                {hidden ? "•••" : `${n > 0 ? "+" : "−"}${amount.format(Math.abs(n))}`}{" "}
                {leg.asset.symbol}
              </span>
            );
          })}
        </span>
        {value !== null && (
          <span className="tabular block text-muted-foreground text-xs">{format(value)}</span>
        )}
      </span>
    </button>
  );
}
