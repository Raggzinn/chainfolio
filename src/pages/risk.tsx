import { Link } from "@tanstack/react-router";
import { ExternalLink, ScanSearch, ShieldCheck } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";
import { Panel } from "@/components/panel";
import { NetworkIcon, TokenIcon } from "@/components/token-icon";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { protocolInfo } from "@/features/defi/model";
import { RiskGauge } from "@/features/portfolio/risk-card";
import { approvalAmount, factorBars, LEVEL_COLOR, riskLevel } from "@/features/risk/model";
import { useAutoScan, useIsScanning, useRiskReport, useScanRisk } from "@/features/risk/queries";
import { usePortfolio } from "@/features/wallets/queries";
import { useMoney } from "@/hooks/use-money";
import type { RiskFactorKey, RiskReport } from "@/lib/api";
import { CHAINS, shortAddress, tokenColor } from "@/lib/chains";
import { formatPercent, formatRelative } from "@/lib/format";
import { gsap, prefersReducedMotion, useGSAP } from "@/lib/gsap";
import { IS_TAURI } from "@/lib/platform";
import { cn } from "@/lib/utils";

export function RiskPage() {
  const { t, i18n } = useTranslation();
  const { data: portfolio } = usePortfolio();
  const { data: report, isPending } = useRiskReport();
  const scan = useScanRisk();
  const scanning = useIsScanning();
  const hasWallets = (portfolio?.wallets.length ?? 0) > 0;
  useAutoScan(hasWallets);
  const locale = i18n.resolvedLanguage ?? "en";

  return (
    <>
      <PageHeader
        title={t("pages.risk.title")}
        description={t("pages.risk.description")}
        actions={
          hasWallets && (
            <>
              {report?.scannedAt && !scanning && (
                <span className="text-muted-foreground text-xs">
                  {t("risk.scannedAt", { when: formatRelative(report.scannedAt, locale) })}
                </span>
              )}
              <Button
                variant="outline"
                onClick={() => scan.mutate()}
                disabled={scanning}
                className="rounded-xl"
              >
                <ScanSearch className={cn(scanning && "animate-pulse")} />
                {scanning ? t("risk.scanning") : t("risk.scan")}
              </Button>
            </>
          )
        }
      />
      {!IS_TAURI ? (
        <EmptyState
          icon={ShieldCheck}
          title={t("risk.empty")}
          description={t("wallets.desktopOnly")}
        />
      ) : isPending || !report ? (
        <Skeleton className="h-80 rounded-2xl" />
      ) : !hasWallets ? (
        <EmptyState
          icon={ShieldCheck}
          title={t("empty.noWallets")}
          description={t("risk.noWalletsHint")}
          action={
            <Button asChild className="rounded-xl">
              <Link to="/wallets">{t("topbar.addWallet")}</Link>
            </Button>
          }
        />
      ) : (
        <RiskReportView report={report} />
      )}
    </>
  );
}

function RiskReportView({ report }: { report: RiskReport }) {
  const { t } = useTranslation();
  const level = riskLevel(report.score);
  return (
    <>
      <div className="grid gap-5 xl:grid-cols-[360px_minmax(0,1fr)]">
        <Panel title={t("dashboard.risk")} subtitle={t("risk.scoreHint")}>
          <RiskGauge
            score={report.score}
            size={168}
            factors={factorBars(report).map((f) => ({
              ...f,
              label: t(`risk.factors.${f.key}.title`),
            }))}
          />
          <p className="mt-5 text-center text-muted-foreground text-xs">
            <span className="font-medium" style={{ color: LEVEL_COLOR[level] }}>
              {t(`dashboard.${level}`)}
            </span>{" "}
            · {t("risk.higherIsRiskier")}
          </p>
        </Panel>
        <FactorList report={report} />
      </div>
      <ApprovalsTable report={report} />
      <div className="grid gap-5 lg:grid-cols-2">
        <FlaggedTokens report={report} />
        <Stablecoins report={report} />
      </div>
      <ExposurePanel report={report} />
    </>
  );
}

function FactorList({ report }: { report: RiskReport }) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const pct = (v: string | number) => formatPercent(Number(v), locale, { signed: false });
  const unlimited = report.approvals.filter((a) => a.unlimited).length;
  const depegged = report.stablecoins.filter((s) => s.depegged);

  const details: Record<RiskFactorKey, string> = {
    concentration: report.concentration.topSymbol
      ? t("risk.factors.concentration.detail", {
          symbol: report.concentration.topSymbol,
          share: pct(report.concentration.topShare),
          hhi: Number(report.concentration.hhi).toFixed(2),
        })
      : t("risk.nothingYet"),
    depeg:
      depegged.length > 0
        ? t("risk.factors.depeg.detailBad", { symbols: depegged.map((s) => s.symbol).join(", ") })
        : t("risk.factors.depeg.detail", { count: report.stablecoins.length }),
    suspicious: t("risk.factors.suspicious.detail", {
      flagged: report.flagged.filter((f) => !f.spam).length,
      spam: report.flagged.filter((f) => f.spam).length,
    }),
    approvals: t("risk.factors.approvals.detail", {
      count: report.approvals.length,
      unlimited,
    }),
    exposure:
      report.exposure.minHealthFactor !== null
        ? t("risk.factors.exposure.detailHealth", {
            share: pct(report.exposure.defiShare),
            hf: Number(report.exposure.minHealthFactor).toFixed(2),
          })
        : t("risk.factors.exposure.detail", { share: pct(report.exposure.defiShare) }),
  };

  return (
    <Panel title={t("risk.breakdown")} subtitle={t("risk.breakdownHint")}>
      <ul className="divide-y divide-border">
        {report.factors.map((f) => {
          const level = riskLevel(f.score);
          return (
            <li key={f.key} className="flex items-start gap-4 py-3.5 first:pt-0 last:pb-0">
              <span
                className="tabular flex size-10 shrink-0 items-center justify-center rounded-xl font-semibold text-sm"
                style={{
                  color: LEVEL_COLOR[level],
                  background: `color-mix(in oklab, ${LEVEL_COLOR[level]} 14%, transparent)`,
                }}
              >
                {f.score}
              </span>
              <div className="min-w-0 flex-1 space-y-0.5">
                <p className="flex items-baseline justify-between gap-3 font-medium text-sm">
                  {t(`risk.factors.${f.key}.title`)}
                  <span className="text-muted-foreground text-xs">
                    {t("risk.weight", { weight: Math.round(Number(f.weight) * 100) })}
                  </span>
                </p>
                <p className="text-muted-foreground text-xs leading-relaxed">
                  {t(`risk.factors.${f.key}.hint`)}
                </p>
                <p className="text-xs">{details[f.key]}</p>
              </div>
            </li>
          );
        })}
      </ul>
    </Panel>
  );
}

function ApprovalsTable({ report }: { report: RiskReport }) {
  const { t, i18n } = useTranslation();
  const scope = useRef<HTMLDivElement>(null);
  const amount = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumFractionDigits: 2 });
  const chains = report.approvalChains.map((c) => CHAINS[c].name).join(", ");

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      gsap.from("[data-approval]", { y: 8, autoAlpha: 0, duration: 0.45, stagger: 0.03 });
    },
    { scope, dependencies: [report.approvals.length], revertOnUpdate: true },
  );

  return (
    <Panel
      title={t("risk.approvals.title")}
      subtitle={t("risk.approvals.hint", { chains })}
      className="pb-2"
    >
      {report.approvals.length === 0 ? (
        <p className="py-8 text-center text-muted-foreground text-sm">
          {report.scannedAt ? t("risk.approvals.none") : t("risk.approvals.notScanned")}
        </p>
      ) : (
        <div ref={scope} className="-mx-2 overflow-x-auto">
          <div className="grid min-w-[640px] grid-cols-[minmax(0,1.2fr)_minmax(0,1.4fr)_minmax(0,1fr)_auto] gap-4 px-3 pb-2 text-[11px] text-muted-foreground uppercase tracking-wider">
            <span>{t("risk.approvals.token")}</span>
            <span>{t("risk.approvals.spender")}</span>
            <span className="text-right">{t("risk.approvals.allowance")}</span>
            <span className="w-20" />
          </div>
          <ul className="min-w-[640px]">
            {report.approvals.map((a) => {
              const value = approvalAmount(a.amount, a.unlimited);
              const symbol = a.tokenSymbol ?? shortAddress(a.token);
              return (
                <li
                  data-approval
                  key={`${a.walletId}-${a.chain}-${a.token}-${a.spender}`}
                  className="grid grid-cols-[minmax(0,1.2fr)_minmax(0,1.4fr)_minmax(0,1fr)_auto] items-center gap-4 rounded-xl px-3 py-2.5 transition-colors hover:bg-accent"
                >
                  <span className="flex min-w-0 items-center gap-2.5">
                    <TokenIcon
                      symbol={symbol}
                      color={tokenColor(symbol, a.chain)}
                      chain={a.chain}
                      size={28}
                    />
                    <span className="truncate font-medium text-sm">{symbol}</span>
                  </span>
                  <span className="min-w-0">
                    <span className="block truncate text-sm">
                      {a.spenderName ?? t("risk.approvals.unknownSpender")}
                    </span>
                    <span className="tabular block truncate text-muted-foreground text-xs">
                      {shortAddress(a.spender)}
                    </span>
                  </span>
                  <span className="text-right text-sm">
                    {value === null ? (
                      <span className="rounded-full bg-danger/12 px-2 py-0.5 font-semibold text-danger text-xs">
                        {a.unlimited ? t("risk.approvals.unlimited") : "—"}
                      </span>
                    ) : (
                      <span className="tabular">
                        {amount.format(value)} {symbol}
                      </span>
                    )}
                  </span>
                  {a.revokeUrl ? (
                    <a
                      href={a.revokeUrl}
                      target="_blank"
                      rel="noreferrer"
                      className="inline-flex w-20 items-center justify-end gap-1 font-medium text-primary text-xs hover:underline"
                    >
                      {t("risk.approvals.revoke")}
                      <ExternalLink className="size-3" />
                    </a>
                  ) : (
                    <span className="w-20" />
                  )}
                </li>
              );
            })}
          </ul>
          <p className="px-3 pt-2 pb-2 text-muted-foreground text-xs">
            {t("risk.approvals.neverSigns")}
          </p>
        </div>
      )}
    </Panel>
  );
}

function FlaggedTokens({ report }: { report: RiskReport }) {
  const { t } = useTranslation();
  const { format } = useMoney();
  return (
    <Panel title={t("risk.flagged.title")} subtitle={t("risk.flagged.hint")}>
      {report.flagged.length === 0 ? (
        <p className="py-6 text-center text-muted-foreground text-sm">{t("risk.flagged.none")}</p>
      ) : (
        <ul className="space-y-2">
          {report.flagged.map((f) => (
            <li
              key={f.assetId}
              className="flex items-center gap-3 rounded-xl bg-muted/50 px-3 py-2.5"
            >
              <TokenIcon
                symbol={f.symbol}
                color={tokenColor(f.symbol, f.chain)}
                chain={f.chain}
                size={28}
              />
              <div className="min-w-0 flex-1">
                <p className="flex items-center gap-2 font-medium text-sm">
                  <span className="truncate">{f.symbol}</span>
                  {f.spam && (
                    <span className="rounded-full bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground">
                      {t("risk.flagged.hidden")}
                    </span>
                  )}
                </p>
                <p className="flex flex-wrap gap-1 pt-0.5">
                  {f.reasons.map((r) => (
                    <span
                      key={r}
                      className={cn(
                        "rounded-full px-1.5 py-0.5 text-[11px]",
                        f.spam ? "bg-danger/12 text-danger" : "bg-warning/12 text-warning",
                      )}
                    >
                      {t(`risk.reasons.${r}`)}
                    </span>
                  ))}
                </p>
              </div>
              <span className="tabular text-sm">
                {f.valueUsd === null ? "—" : format(Number(f.valueUsd))}
              </span>
            </li>
          ))}
        </ul>
      )}
    </Panel>
  );
}

function Stablecoins({ report }: { report: RiskReport }) {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const locale = i18n.resolvedLanguage ?? "en";
  const price = new Intl.NumberFormat(locale, {
    minimumFractionDigits: 4,
    maximumFractionDigits: 4,
  });
  return (
    <Panel title={t("risk.stables.title")} subtitle={t("risk.stables.hint")}>
      {report.stablecoins.length === 0 ? (
        <p className="py-6 text-center text-muted-foreground text-sm">{t("risk.stables.none")}</p>
      ) : (
        <ul className="space-y-1">
          {report.stablecoins.map((s) => (
            <li
              key={`${s.chain}-${s.symbol}`}
              className="flex items-center gap-3 rounded-xl px-2 py-2"
            >
              <TokenIcon
                symbol={s.symbol}
                color={tokenColor(s.symbol, s.chain)}
                chain={s.chain}
                size={28}
              />
              <span className="min-w-0 flex-1 truncate font-medium text-sm">{s.symbol}</span>
              <span className="tabular text-muted-foreground text-xs">
                ${price.format(Number(s.priceUsd))}
              </span>
              <span
                className={cn(
                  "tabular w-16 rounded-full px-2 py-0.5 text-center font-medium text-xs",
                  s.depegged ? "bg-danger/12 text-danger" : "bg-success/12 text-success",
                )}
              >
                {s.depegged
                  ? formatPercent(Number(s.deviation), locale, { signed: false })
                  : t("risk.stables.pegged")}
              </span>
              <span className="tabular w-24 text-right text-sm">{format(Number(s.valueUsd))}</span>
            </li>
          ))}
        </ul>
      )}
    </Panel>
  );
}

function ExposurePanel({ report }: { report: RiskReport }) {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const locale = i18n.resolvedLanguage ?? "en";
  const scope = useRef<HTMLDivElement>(null);
  const pct = (v: string) => formatPercent(Number(v), locale, { signed: false });

  useGSAP(
    () => {
      gsap.from("[data-share]", {
        scaleX: 0,
        transformOrigin: "left center",
        duration: prefersReducedMotion() ? 0 : 1,
        ease: "expo.out",
        stagger: 0.06,
      });
    },
    { scope, dependencies: [report.exposure.chains.length], revertOnUpdate: true },
  );

  const bar = (
    key: string,
    label: React.ReactNode,
    share: string,
    value: string,
    color: string,
  ) => (
    <li key={key} className="space-y-1.5">
      <div className="flex items-center justify-between gap-3 text-xs">
        <span className="flex min-w-0 items-center gap-2">{label}</span>
        <span className="tabular text-muted-foreground">
          {format(Number(value))} ·{" "}
          <span className="font-medium text-foreground">{pct(share)}</span>
        </span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-muted">
        <div
          data-share
          className="h-full rounded-full"
          style={{ width: `${Math.min(100, Number(share) * 100)}%`, background: color }}
        />
      </div>
    </li>
  );

  return (
    <Panel title={t("risk.exposure.title")} subtitle={t("risk.exposure.hint")}>
      <div ref={scope} className="grid gap-8 lg:grid-cols-2">
        <div className="space-y-3">
          <p className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
            {t("risk.exposure.chains")}
          </p>
          <ul className="space-y-3">
            {report.exposure.chains.map((c) =>
              bar(
                c.key,
                <>
                  <NetworkIcon chain={c.key} size={16} />
                  {CHAINS[c.key].name}
                </>,
                c.share,
                c.valueUsd,
                CHAINS[c.key].color,
              ),
            )}
          </ul>
        </div>
        <div className="space-y-3">
          <p className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
            {t("risk.exposure.protocols")}
          </p>
          {report.exposure.protocols.length === 0 ? (
            <p className="text-muted-foreground text-sm">{t("risk.exposure.noProtocols")}</p>
          ) : (
            <ul className="space-y-3">
              {report.exposure.protocols.map((p) => {
                const info = protocolInfo(p.key);
                return bar(p.key, info.name, p.share, p.valueUsd, info.color);
              })}
            </ul>
          )}
        </div>
      </div>
    </Panel>
  );
}
