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

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["day", 86_400_000],
  ["hour", 3_600_000],
  ["minute", 60_000],
];

/** "3 minutes ago" / "há 3 minutos" for a past timestamp in ms. */
export function formatRelative(timestamp: number, locale: string, now = Date.now()): string {
  const elapsed = Math.max(0, now - timestamp);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  for (const [unit, ms] of UNITS) {
    if (elapsed >= ms) return rtf.format(-Math.floor(elapsed / ms), unit);
  }
  return rtf.format(0, "second");
}
