import { Link } from "@tanstack/react-router";
import { ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ChangePill } from "@/components/change-pill";
import { Panel } from "@/components/panel";
import { Sparkline } from "@/components/sparkline";
import { TokenIcon } from "@/components/token-icon";
import { assetValue, CHAINS, DEMO_ASSETS, demoSparkline } from "@/features/demo/data";
import { useMoney } from "@/hooks/use-money";

export function TopAssetsCard() {
  const { t, i18n } = useTranslation();
  const { format, hidden } = useMoney();
  const assets = [...DEMO_ASSETS].sort((a, b) => assetValue(b) - assetValue(a)).slice(0, 6);
  const amount = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumFractionDigits: 4 });

  return (
    <Panel
      title={t("dashboard.topAssets")}
      action={
        <Link
          to="/assets"
          className="group flex items-center gap-0.5 font-medium text-primary text-xs"
        >
          {t("dashboard.viewAll")}
          <ChevronRight className="size-3.5 transition-transform group-hover:translate-x-0.5" />
        </Link>
      }
      className="pb-2"
    >
      <div className="-mx-2">
        <div className="grid grid-cols-[minmax(0,1.6fr)_1fr_1fr_110px_1fr] gap-4 px-3 pb-2 text-[11px] text-muted-foreground uppercase tracking-wider">
          <span>{t("dashboard.columns.asset")}</span>
          <span className="text-right">{t("dashboard.columns.price")}</span>
          <span className="text-right">{t("dashboard.columns.change")}</span>
          <span className="text-center">{t("dashboard.columns.trend")}</span>
          <span className="text-right">{t("dashboard.columns.value")}</span>
        </div>
        <ul>
          {assets.map((a) => (
            <li
              key={a.symbol}
              className="group grid grid-cols-[minmax(0,1.6fr)_1fr_1fr_110px_1fr] items-center gap-4 rounded-xl px-3 py-2.5 transition-all duration-200 hover:bg-accent"
            >
              <div className="flex min-w-0 items-center gap-3">
                <TokenIcon
                  symbol={a.symbol}
                  color={a.color}
                  chain={a.chain}
                  className="transition-transform duration-300 group-hover:scale-110"
                />
                <div className="min-w-0">
                  <p className="truncate font-medium text-sm">{a.name}</p>
                  <p className="truncate text-muted-foreground text-xs">
                    {hidden ? "•••" : amount.format(a.balance)} {a.symbol} · {CHAINS[a.chain].name}
                  </p>
                </div>
              </div>
              <span className="tabular text-right text-sm">
                {format(a.price, { ignorePrivacy: true })}
              </span>
              <span className="text-right">
                <ChangePill value={a.change24h} />
              </span>
              <span className="flex justify-center">
                <Sparkline data={demoSparkline(a)} positive={a.change24h >= 0} />
              </span>
              <span className="tabular text-right font-semibold text-sm">
                {format(assetValue(a))}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </Panel>
  );
}
