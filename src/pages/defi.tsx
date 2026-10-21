import { Link } from "@tanstack/react-router";
import { Info, Layers, RefreshCw } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";
import { Panel } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { NetworkIcon, TokenIcon } from "@/components/token-icon";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  groupByProtocol,
  HEALTH_COLOR,
  healthLevel,
  type ProtocolGroup,
  positionKey,
  summarize,
} from "@/features/defi/model";
import { usePositions } from "@/features/defi/queries";
import { useIsSyncing, usePortfolio, useRefreshBalances } from "@/features/wallets/queries";
import { useMoney } from "@/hooks/use-money";
import type { ChainId, Position, PositionToken } from "@/lib/api";
import { CHAINS, tokenColor } from "@/lib/chains";
import { formatPercent } from "@/lib/format";
import { gsap, prefersReducedMotion, useGSAP } from "@/lib/gsap";
import { IS_TAURI } from "@/lib/platform";
import { cn } from "@/lib/utils";

const FILTERS = ["all", "lending", "liquidity", "staking"] as const;
type Filter = (typeof FILTERS)[number];

export function DefiPage() {
  const { t } = useTranslation();
  const { data: portfolio } = usePortfolio();
  const { data: positions, isPending } = usePositions();
  const refresh = useRefreshBalances();
  const syncing = useIsSyncing();
  const [filter, setFilter] = useState<Filter>("all");
  const hasWallets = (portfolio?.wallets.length ?? 0) > 0;

  const visible = useMemo(
    () => (positions ?? []).filter((p) => filter === "all" || p.kind === filter),
    [positions, filter],
  );
  const groups = useMemo(() => groupByProtocol(visible), [visible]);

  return (
    <>
      <PageHeader
        title={t("pages.defi.title")}
        description={t("pages.defi.description")}
        actions={
          hasWallets && (
            <Button
              variant="outline"
              onClick={() => refresh.mutate(undefined)}
              disabled={syncing}
              className="rounded-xl"
            >
              <RefreshCw className={cn(syncing && "animate-spin")} />
              {syncing ? t("wallets.syncing") : t("topbar.sync")}
            </Button>
          )
        }
      />
      {!IS_TAURI ? (
        <EmptyState icon={Layers} title={t("defi.empty")} description={t("wallets.desktopOnly")} />
      ) : isPending ? (
        <Skeleton className="h-64 rounded-2xl" />
      ) : !hasWallets ? (
        <EmptyState
          icon={Layers}
          title={t("empty.noWallets")}
          description={t("defi.noWalletsHint")}
          action={
            <Button asChild className="rounded-xl">
              <Link to="/wallets">{t("topbar.addWallet")}</Link>
            </Button>
          }
        />
      ) : (positions ?? []).length === 0 ? (
        <EmptyState
          icon={Layers}
          title={syncing ? t("wallets.syncing") : t("defi.empty")}
          description={t("defi.emptyHint")}
        />
      ) : (
        <>
          <SummaryTiles positions={positions ?? []} />
          <SegmentedControl
            label={t("defi.filter")}
            options={FILTERS}
            labels={Object.fromEntries(FILTERS.map((f) => [f, t(`defi.kinds.${f}`)]))}
            value={filter}
            onChange={setFilter}
            className="self-start"
          />
          <ProtocolList groups={groups} />
        </>
      )}
    </>
  );
}

function SummaryTiles({ positions }: { positions: Position[] }) {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const locale = i18n.resolvedLanguage ?? "en";
  const summary = summarize(positions);
  const hf = summary.minHealthFactor;

  const tiles = [
    { label: t("defi.summary.total"), value: format(summary.total) },
    {
      label: t("defi.summary.apy"),
      value: summary.apy === null ? "—" : formatPercent(summary.apy, locale, { signed: false }),
    },
    { label: t("defi.summary.yearly"), value: format(summary.yearlyYield) },
    {
      label: t("defi.summary.health"),
      value: hf === null ? "—" : hf.toFixed(2),
      color: hf === null ? undefined : HEALTH_COLOR[healthLevel(hf)],
    },
  ];
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      {tiles.map((tile) => (
        <div key={tile.label} className="surface rounded-2xl px-5 py-4">
          <p className="text-muted-foreground text-xs">{tile.label}</p>
          <p
            className="tabular mt-1 font-semibold text-2xl tracking-tight"
            style={tile.color ? { color: tile.color } : undefined}
          >
            {tile.value}
          </p>
        </div>
      ))}
    </div>
  );
}

function ProtocolList({ groups }: { groups: ProtocolGroup[] }) {
  const { t } = useTranslation();
  const { format } = useMoney();
  const scope = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      gsap.from("[data-position]", { y: 10, autoAlpha: 0, duration: 0.5, stagger: 0.04 });
    },
    { scope, dependencies: [groups.length], revertOnUpdate: true },
  );

  if (groups.length === 0) {
    return (
      <p className="py-10 text-center text-muted-foreground text-sm">{t("defi.noneOfKind")}</p>
    );
  }
  return (
    <div ref={scope} className="grid gap-5">
      {groups.map((group) => (
        <Panel
          key={group.protocol}
          title={
            <span className="flex items-center gap-2.5">
              <span
                aria-hidden="true"
                className="size-2.5 rounded-full"
                style={{ background: group.info.color }}
              />
              {group.info.name}
            </span>
          }
          subtitle={t("defi.positionsCount", { count: group.positions.length })}
          action={<span className="tabular font-semibold text-lg">{format(group.value)}</span>}
          className="pb-2"
        >
          <ul className="-mx-2 divide-y divide-border">
            {group.positions.map((p) => (
              <PositionRow key={positionKey(p)} position={p} />
            ))}
          </ul>
        </Panel>
      ))}
    </div>
  );
}

function PositionRow({ position: p }: { position: Position }) {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const locale = i18n.resolvedLanguage ?? "en";
  const value = p.valueUsd === null ? null : Number(p.valueUsd);

  return (
    <li
      data-position
      className="grid gap-3 px-3 py-3.5 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1.6fr)_auto_auto]"
    >
      <div className="flex min-w-0 items-center gap-3">
        <NetworkIcon chain={p.chain} size={28} />
        <div className="min-w-0">
          <p className="truncate font-medium text-sm">{p.name}</p>
          <p className="flex flex-wrap items-center gap-x-2 text-muted-foreground text-xs">
            <span>{CHAINS[p.chain].name}</span>
            <span>·</span>
            <span>{t(`defi.kinds.${p.kind}`)}</span>
            {p.inBalances && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="inline-flex items-center gap-1 text-foreground/80">
                    <Info className="size-3" />
                    {t("defi.inHoldings")}
                  </span>
                </TooltipTrigger>
                <TooltipContent>{t("defi.inHoldingsHint")}</TooltipContent>
              </Tooltip>
            )}
          </p>
        </div>
      </div>

      <div className="flex min-w-0 flex-col gap-1.5">
        <TokenChips tokens={p.tokens} chain={p.chain} />
        {p.range && <RangeLine position={p} />}
      </div>

      <div className="flex items-center gap-4 text-xs lg:justify-end">
        <Metric label={t("defi.apy")}>
          {p.apy === null ? "—" : formatPercent(Number(p.apy), locale, { signed: false })}
        </Metric>
        {p.healthFactor !== null && <HealthPill value={Number(p.healthFactor)} />}
        {p.kind === "liquidity" && (
          <Metric label={t("defi.il")}>
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="cursor-help underline decoration-dotted underline-offset-2">
                  {p.impermanentLoss === null
                    ? t("defi.na")
                    : formatPercent(Number(p.impermanentLoss), locale)}
                </span>
              </TooltipTrigger>
              <TooltipContent className="max-w-64">{t("defi.ilHint")}</TooltipContent>
            </Tooltip>
          </Metric>
        )}
      </div>

      <span className="tabular self-center text-right font-semibold text-sm">
        {value === null ? "—" : format(value)}
      </span>
    </li>
  );
}

function Metric({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <span className="flex flex-col">
      <span className="text-[11px] text-muted-foreground uppercase tracking-wider">{label}</span>
      <span className="tabular font-medium">{children}</span>
    </span>
  );
}

function HealthPill({ value }: { value: number }) {
  const { t } = useTranslation();
  const level = healthLevel(value);
  return (
    <Metric label={t("defi.health")}>
      <span
        className="rounded-full px-2 py-0.5 font-semibold"
        style={{
          color: HEALTH_COLOR[level],
          background: `color-mix(in oklab, ${HEALTH_COLOR[level]} 14%, transparent)`,
        }}
      >
        {value.toFixed(2)}
      </span>
    </Metric>
  );
}

function TokenChips({ tokens, chain }: { tokens: PositionToken[]; chain: ChainId }) {
  const { t, i18n } = useTranslation();
  const { hidden } = useMoney();
  const amount = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumFractionDigits: 4 });
  return (
    <div className="flex flex-wrap gap-1.5">
      {tokens.map((token) => (
        <span
          key={`${token.role}-${token.contract ?? token.symbol}`}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-lg bg-muted px-2 py-1 text-xs",
            token.role === "borrowed" && "text-danger",
          )}
        >
          <TokenIcon symbol={token.symbol} color={tokenColor(token.symbol, chain)} size={16} />
          <span className="text-muted-foreground">{t(`defi.roles.${token.role}`)}</span>
          <span className="tabular font-medium">
            {token.role === "borrowed" ? "−" : ""}
            {hidden ? "•••" : amount.format(Number(token.amount))} {token.symbol}
          </span>
        </span>
      ))}
    </div>
  );
}

function RangeLine({ position: p }: { position: Position }) {
  const { t, i18n } = useTranslation();
  if (!p.range) return null;
  const price = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumSignificantDigits: 5 });
  const pooled = p.tokens.filter((tk) => tk.role === "pooled");
  const [base, quote] = pooled;
  const pair = base && quote ? `${quote.symbol}/${base.symbol}` : "";
  return (
    <p className="flex flex-wrap items-center gap-2 text-muted-foreground text-xs">
      <span
        className={cn(
          "rounded-full px-2 py-0.5 font-medium",
          p.range.inRange ? "bg-success/12 text-success" : "bg-warning/12 text-warning",
        )}
      >
        {p.range.inRange ? t("defi.inRange") : t("defi.outOfRange")}
      </span>
      <span className="tabular">
        {price.format(Number(p.range.lower))} – {price.format(Number(p.range.upper))} {pair}
      </span>
      <span className="tabular">
        · {t("defi.current")} {price.format(Number(p.range.current))}
      </span>
    </p>
  );
}
