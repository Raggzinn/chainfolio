import { Link } from "@tanstack/react-router";
import { ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { NetworkIcon } from "@/components/token-icon";
import { useMoney } from "@/hooks/use-money";
import { formatPercent } from "@/lib/format";
import { positionKey, protocolInfo, summarize } from "./model";
import { usePositions } from "./queries";

/** Largest DeFi and staking positions, for the dashboard. */
export function DefiCard() {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const { data: positions = [] } = usePositions();
  const summary = summarize(positions);
  const top = positions.slice(0, 4);
  const locale = i18n.resolvedLanguage ?? "en";

  return (
    <Panel
      title={t("nav.defi")}
      subtitle={
        positions.length > 0
          ? `${format(summary.total)}${summary.apy === null ? "" : ` · ${t("defi.apy")} ${formatPercent(summary.apy, locale, { signed: false })}`}`
          : undefined
      }
      action={
        <Link
          to="/defi"
          className="group flex items-center gap-0.5 font-medium text-primary text-xs"
        >
          {t("dashboard.viewAll")}
          <ChevronRight className="size-3.5 transition-transform group-hover:translate-x-0.5" />
        </Link>
      }
    >
      {top.length === 0 ? (
        <p className="py-8 text-center text-muted-foreground text-sm">{t("defi.empty")}</p>
      ) : (
        <ul className="-mx-2">
          {top.map((p) => {
            const info = protocolInfo(p.protocol);
            return (
              <li
                key={positionKey(p)}
                className="flex items-center gap-3 rounded-xl px-2 py-2.5 transition-colors hover:bg-accent"
              >
                <NetworkIcon chain={p.chain} size={28} />
                <div className="min-w-0 flex-1">
                  <p className="truncate font-medium text-sm">{p.name}</p>
                  <p className="flex items-center gap-1.5 text-muted-foreground text-xs">
                    <span className="size-1.5 rounded-full" style={{ background: info.color }} />
                    {info.name}
                  </p>
                </div>
                <span className="tabular font-semibold text-sm">
                  {p.valueUsd === null ? "—" : format(Number(p.valueUsd))}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </Panel>
  );
}
