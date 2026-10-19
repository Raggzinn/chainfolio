import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { formatCurrency } from "@/lib/format";
import { type Currency, usePreferences } from "@/stores/preferences";

/**
 * Placeholder USD conversion rates until the FX provider lands in the backend.
 * Only used to render demo data in the selected base currency.
 */
const USD_RATES: Record<Currency, number> = { USD: 1, BRL: 5.42, EUR: 0.92 };

export function useMoney() {
  const { i18n } = useTranslation();
  const currency = usePreferences((s) => s.currency);
  const hidden = usePreferences((s) => s.privacyMode);
  const locale = i18n.resolvedLanguage ?? "en";
  const rate = USD_RATES[currency];

  const format = useCallback(
    (usd: number, opts: { compact?: boolean; ignorePrivacy?: boolean } = {}) =>
      formatCurrency(usd * rate, currency, locale, {
        compact: opts.compact,
        hidden: hidden && !opts.ignorePrivacy,
      }),
    [currency, locale, rate, hidden],
  );

  return { format, currency, locale, hidden, rate };
}
