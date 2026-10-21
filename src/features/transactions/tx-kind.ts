import {
  ArrowDownLeft,
  ArrowLeftRight,
  ArrowUpRight,
  Ban,
  Coins,
  FileCode2,
  Gift,
  type LucideIcon,
  Repeat,
  ShieldAlert,
  Sparkles,
  Sprout,
} from "lucide-react";
import type { TxItem, TxKind } from "@/lib/api";

export const KIND_STYLE: Record<TxKind, { icon: LucideIcon; tone: string }> = {
  receive: { icon: ArrowDownLeft, tone: "bg-success/12 text-success" },
  send: { icon: ArrowUpRight, tone: "bg-muted text-foreground" },
  swap: { icon: ArrowLeftRight, tone: "bg-primary/12 text-primary" },
  internal: { icon: Repeat, tone: "bg-muted text-muted-foreground" },
  contract: { icon: FileCode2, tone: "bg-muted text-muted-foreground" },
  failed: { icon: Ban, tone: "bg-danger/12 text-danger" },
  income: { icon: Coins, tone: "bg-success/12 text-success" },
  reward: { icon: Sprout, tone: "bg-success/12 text-success" },
  airdrop: { icon: Sparkles, tone: "bg-success/12 text-success" },
  gift: { icon: Gift, tone: "bg-muted text-foreground" },
  spam: { icon: ShieldAlert, tone: "bg-warning/12 text-warning" },
};

/** Symbols moved in each direction, e.g. `{ out: "USDC", in: "ETH" }`. */
export function legSymbols(tx: TxItem) {
  const join = (positive: boolean) =>
    [
      ...new Set(
        tx.legs.filter((l) => Number(l.amount) > 0 === positive).map((l) => l.asset.symbol),
      ),
    ].join(", ");
  return { out: join(false), in: join(true) };
}

/** Value of what the wallet received (or sent, when nothing came in), in USD. */
export function headlineValue(tx: TxItem): number | null {
  const pick = (positive: boolean) =>
    tx.legs.filter((l) => Number(l.amount) > 0 === positive && l.valueUsd !== null);
  const legs = pick(true).length > 0 ? pick(true) : pick(false);
  if (legs.length === 0) return null;
  return legs.reduce((sum, l) => sum + Math.abs(Number(l.valueUsd)), 0);
}
