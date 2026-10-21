import type { TFunction } from "i18next";
import type { AlertCondition, AlertEvent } from "@/lib/api";
import { formatPercent, MASK } from "@/lib/format";

export interface Describer {
  t: TFunction;
  locale: string;
  /** Wallet label by id; `null` means "any wallet". */
  wallet: (id: number | null) => string;
  /** Hide amounts of money the user holds (prices stay visible). */
  hidden?: boolean;
}

/** Prices keep more decimals below $1 so a 0.975 depeg doesn't read as $0.98. */
export function formatUsd(value: number, locale: string): string {
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: Math.abs(value) < 1 ? 4 : 2,
  }).format(value);
}

const pct = (ratio: string | number, locale: string) =>
  formatPercent(Number(ratio), locale, { signed: false });

/** "BTC goes above $100,000.00" */
export function describeCondition(c: AlertCondition, d: Describer): string {
  const { t, locale } = d;
  switch (c.kind) {
    case "priceAbove":
      return t("alerts.rule.priceAbove", {
        symbol: c.symbol,
        price: formatUsd(Number(c.price), locale),
      });
    case "priceBelow":
      return t("alerts.rule.priceBelow", {
        symbol: c.symbol,
        price: formatUsd(Number(c.price), locale),
      });
    case "portfolioChange": {
      const percent = pct(c.ratio, locale);
      if (c.direction === "up") return t("alerts.rule.portfolioUp", { percent });
      if (c.direction === "down") return t("alerts.rule.portfolioDown", { percent });
      return t("alerts.rule.portfolioEither", { percent });
    }
    case "balanceChange": {
      const params = {
        amount: d.hidden ? MASK : formatUsd(Number(c.usd), locale),
        wallet: d.wallet(c.walletId),
      };
      if (c.direction === "up") return t("alerts.rule.balanceUp", params);
      if (c.direction === "down") return t("alerts.rule.balanceDown", params);
      return t("alerts.rule.balanceEither", params);
    }
    case "depeg":
      return t("alerts.rule.depeg", { percent: pct(c.ratio, locale) });
  }
}

/** "BTC reached $101,234.00" */
export function describeEvent(e: AlertEvent, d: Describer): string {
  const { t, locale } = d;
  const value = Number(e.value);
  switch (e.condition.kind) {
    case "priceAbove":
    case "priceBelow":
      return t(
        e.condition.kind === "priceAbove" ? "alerts.event.priceAbove" : "alerts.event.priceBelow",
        {
          symbol: e.condition.symbol,
          price: formatUsd(value, locale),
        },
      );
    case "portfolioChange":
      return t("alerts.event.portfolio", {
        percent: formatPercent(value, locale),
      });
    case "balanceChange": {
      const params = {
        amount: d.hidden ? MASK : formatUsd(Math.abs(value), locale),
        wallet: d.wallet(e.subject === null ? null : Number(e.subject)),
      };
      return value >= 0
        ? t("alerts.event.balanceIn", params)
        : t("alerts.event.balanceOut", params);
    }
    case "depeg":
      return t("alerts.event.depeg", {
        symbol: e.subject ?? "?",
        price: formatUsd(value, locale),
      });
  }
}
