import { assetValue, DEMO_ASSETS, demoSparkline } from "@/features/demo/data";
import type { ChainId, Portfolio } from "@/lib/api";
import { tokenColor } from "@/lib/chains";

/** One asset as the dashboard cards draw it, from demo data or real balances. */
export interface AssetRow {
  key: string;
  symbol: string;
  name: string;
  chain: ChainId;
  color: string;
  logoUrl?: string | null;
  balance: number;
  /** Part of `balance` that can't be spent (XRP Ledger reserve). */
  locked?: number;
  price: number;
  /** Ratio, 0.05 = +5%; `null` when unknown. */
  change24h: number | null;
  value: number;
  /** Recent price points, oldest first. Real wallets get these with history sync. */
  sparkline?: number[];
}

export function demoAssetRows(): AssetRow[] {
  return DEMO_ASSETS.map((a) => ({
    key: a.symbol,
    symbol: a.symbol,
    name: a.name,
    chain: a.chain,
    color: a.color,
    balance: a.balance,
    price: a.price,
    change24h: a.change24h,
    value: assetValue(a),
    sparkline: demoSparkline(a),
  }));
}

/** Converts the backend's decimal strings into display rows, largest first. */
export function liveAssetRows(portfolio: Portfolio): AssetRow[] {
  return portfolio.assets
    .map((a) => ({
      key: a.id,
      symbol: a.symbol,
      name: a.name,
      chain: a.chain,
      color: tokenColor(a.symbol, a.chain),
      logoUrl: a.logoUrl,
      balance: Number(a.amount),
      locked: a.locked === null ? undefined : Number(a.locked),
      price: Number(a.priceUsd),
      change24h: a.change24h === null ? null : Number(a.change24h),
      value: Number(a.valueUsd),
    }))
    .sort((a, b) => b.value - a.value);
}

export const rowsTotal = (rows: AssetRow[]) => rows.reduce((sum, r) => sum + r.value, 0);
