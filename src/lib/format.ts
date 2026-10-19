import type { Currency } from "@/stores/preferences";

export const MASK = "•••••";

export function formatCurrency(
  value: number,
  currency: Currency,
  locale: string,
  { hidden = false, compact = false }: { hidden?: boolean; compact?: boolean } = {},
): string {
  if (hidden) return MASK;
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
    notation: compact ? "compact" : "standard",
    maximumFractionDigits: compact ? 1 : 2,
  }).format(value);
}

export function formatPercent(value: number, locale: string, { signed = true } = {}): string {
  return new Intl.NumberFormat(locale, {
    style: "percent",
    maximumFractionDigits: 2,
    signDisplay: signed ? "exceptZero" : "auto",
  }).format(value);
}
