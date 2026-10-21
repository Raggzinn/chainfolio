import type { Position } from "@/lib/api";

export interface ProtocolInfo {
  name: string;
  color: string;
}

/** Display metadata for the protocols the backend reads. */
export const PROTOCOLS: Record<string, ProtocolInfo> = {
  "aave-v3": { name: "Aave v3", color: "#9391F7" },
  "compound-v3": { name: "Compound v3", color: "#00D395" },
  "uniswap-v3": { name: "Uniswap v3", color: "#FF007A" },
  lido: { name: "Lido", color: "#00A3FF" },
  "rocket-pool": { name: "Rocket Pool", color: "#FF6E30" },
  coinbase: { name: "Coinbase", color: "#0052FF" },
  marinade: { name: "Marinade", color: "#308D8A" },
  jito: { name: "Jito", color: "#5AC587" },
  benqi: { name: "BENQI", color: "#00B3ED" },
  "solana-staking": { name: "Solana staking", color: "#9945FF" },
};

export function protocolInfo(id: string): ProtocolInfo {
  return PROTOCOLS[id] ?? { name: id, color: "#64748B" };
}

export interface ProtocolGroup {
  protocol: string;
  info: ProtocolInfo;
  value: number;
  positions: Position[];
}

/** Stable React key: one position per wallet, chain, protocol and name. */
export const positionKey = (p: Position) => `${p.walletId}-${p.chain}-${p.protocol}-${p.name}`;

const value = (p: Position) => (p.valueUsd === null ? 0 : Number(p.valueUsd));

/** Positions grouped by protocol, largest group and position first. */
export function groupByProtocol(positions: Position[]): ProtocolGroup[] {
  const groups = new Map<string, ProtocolGroup>();
  for (const p of positions) {
    const group = groups.get(p.protocol) ?? {
      protocol: p.protocol,
      info: protocolInfo(p.protocol),
      value: 0,
      positions: [],
    };
    group.value += value(p);
    group.positions.push(p);
    groups.set(p.protocol, group);
  }
  return [...groups.values()]
    .map((g) => ({ ...g, positions: [...g.positions].sort((a, b) => value(b) - value(a)) }))
    .sort((a, b) => b.value - a.value);
}

export interface DefiSummary {
  total: number;
  /** Value-weighted APY of positions that report one; `null` if none do. */
  apy: number | null;
  yearlyYield: number;
  minHealthFactor: number | null;
}

export function summarize(positions: Position[]): DefiSummary {
  let total = 0;
  let earning = 0;
  let yearlyYield = 0;
  let minHealthFactor: number | null = null;
  for (const p of positions) {
    const v = value(p);
    total += v;
    if (p.apy !== null) {
      earning += v;
      yearlyYield += v * Number(p.apy);
    }
    if (p.healthFactor !== null) {
      const hf = Number(p.healthFactor);
      minHealthFactor = minHealthFactor === null ? hf : Math.min(minHealthFactor, hf);
    }
  }
  return { total, apy: earning > 0 ? yearlyYield / earning : null, yearlyYield, minHealthFactor };
}

export type HealthLevel = "safe" | "watch" | "danger";

/** Lending positions are liquidated below 1. */
export function healthLevel(healthFactor: number): HealthLevel {
  if (healthFactor >= 2) return "safe";
  if (healthFactor >= 1.25) return "watch";
  return "danger";
}

export const HEALTH_COLOR: Record<HealthLevel, string> = {
  safe: "var(--success)",
  watch: "var(--warning)",
  danger: "var(--danger)",
};
