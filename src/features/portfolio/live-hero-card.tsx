import { Link } from "@tanstack/react-router";
import { LineChart, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ChangePill } from "@/components/change-pill";
import { AnimatedNumber } from "@/components/effects/animated-number";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useIsSyncing, useRefreshBalances } from "@/features/wallets/queries";
import { useMoney } from "@/hooks/use-money";
import type { Portfolio } from "@/lib/api";
import { CHAINS } from "@/lib/chains";
import { formatPercent, formatRelative } from "@/lib/format";
import { cn } from "@/lib/utils";

/** Total value of the tracked wallets, from stored balances. */
export function LiveHeroCard({ portfolio }: { portfolio: Portfolio }) {
  const { t, i18n } = useTranslation();
  const { format, hidden } = useMoney();
  const refresh = useRefreshBalances();
  const syncing = useIsSyncing();
  const locale = i18n.resolvedLanguage ?? "en";

  const total = Number(portfolio.totalUsd);
  const delta = Number(portfolio.change24hUsd);
  const change = portfolio.change24h === null ? null : Number(portfolio.change24h);
  const chains = portfolio.chains
    .map((c) => ({ ...c, value: Number(c.valueUsd) }))
    .filter((c) => c.value > 0);

  return (
    <section className="surface relative flex flex-col overflow-hidden rounded-3xl">
      <div className="flex flex-wrap items-start justify-between gap-6 p-6">
        <div className="space-y-3">
          <p className="font-medium text-muted-foreground text-sm">{t("dashboard.totalValue")}</p>
          <AnimatedNumber
            value={total}
            format={(n) => format(n, { ignorePrivacy: true })}
            hidden={hidden}
            className="tabular block font-semibold text-[52px] leading-none tracking-[-0.035em]"
          />
          {change !== null && (
            <div className="flex items-center gap-2.5 text-sm">
              <ChangePill value={change} size="md" />
              <span className="tabular text-muted-foreground">
                <span className={cn("font-medium", delta >= 0 ? "text-success" : "text-danger")}>
                  {delta >= 0 ? "+" : ""}
                  {format(delta)}
                </span>{" "}
                {t("filters.ranges.1D")}
              </span>
            </div>
          )}
        </div>

        <div className="flex items-center gap-3 text-muted-foreground text-xs">
          <span>
            {syncing
              ? t("wallets.syncing")
              : portfolio.syncedAt
                ? t("dashboard.updated", { when: formatRelative(portfolio.syncedAt, locale) })
                : t("wallets.neverSynced")}
          </span>
          <Button
            variant="outline"
            size="icon-sm"
            aria-label={t("topbar.sync")}
            disabled={syncing}
            onClick={() => refresh.mutate(undefined)}
            className="rounded-lg"
          >
            <RefreshCw className={cn(syncing && "animate-spin")} />
          </Button>
        </div>
      </div>

      <div className="mt-auto space-y-3 px-6 pb-5">
        {chains.length > 0 && (
          <>
            <div className="flex h-2.5 gap-1 overflow-hidden rounded-full">
              {chains.map((c) => (
                <Tooltip key={c.chain}>
                  <TooltipTrigger asChild>
                    <span
                      className="h-full rounded-full"
                      style={{ flexGrow: c.value, background: CHAINS[c.chain].color }}
                    />
                  </TooltipTrigger>
                  <TooltipContent>
                    {CHAINS[c.chain].name} · {format(c.value)}
                  </TooltipContent>
                </Tooltip>
              ))}
            </div>
            <ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs">
              {chains.map((c) => (
                <li key={c.chain} className="flex items-center gap-1.5">
                  <span
                    className="size-2 rounded-full"
                    style={{ background: CHAINS[c.chain].color }}
                  />
                  <span>{CHAINS[c.chain].name}</span>
                  <span className="tabular text-muted-foreground">
                    {formatPercent(total ? c.value / total : 0, locale, { signed: false })}
                  </span>
                </li>
              ))}
            </ul>
          </>
        )}
        <p className="flex items-center gap-1.5 text-muted-foreground text-xs">
          <LineChart className="size-3.5" />
          {t("dashboard.historySoon")}
        </p>
      </div>

      <div className="grid grid-cols-3 divide-x divide-border border-t">
        <Stat label={t("dashboard.walletsCount", { count: portfolio.wallets.length })}>
          <Link to="/wallets" className="text-primary text-sm hover:underline">
            {t("dashboard.manageWallets")}
          </Link>
        </Stat>
        <Stat label={t("dashboard.assetsCount", { count: portfolio.assets.length })}>
          <span className="text-muted-foreground text-sm">
            {t("dashboard.chainsCount", { count: chains.length })}
          </span>
        </Stat>
        <Stat label={t("dashboard.unpriced")}>
          <span className="text-muted-foreground text-sm">
            {t("dashboard.unpricedCount", { count: portfolio.unpricedCount })}
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
      <div className="tabular font-semibold text-lg tracking-tight">{children}</div>
    </div>
  );
}
