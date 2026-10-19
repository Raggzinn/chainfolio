/**
 * Static sample portfolio used by demo mode and screenshots. Replaced by real
 * queries once wallet sync lands; nothing here touches the network.
 */

export type ChainId = "ethereum" | "solana" | "bitcoin" | "arbitrum" | "base" | "optimism";

export interface Chain {
  id: ChainId;
  name: string;
  color: string;
}

export const CHAINS: Record<ChainId, Chain> = {
  ethereum: { id: "ethereum", name: "Ethereum", color: "#627EEA" },
  bitcoin: { id: "bitcoin", name: "Bitcoin", color: "#F7931A" },
  solana: { id: "solana", name: "Solana", color: "#14F195" },
  arbitrum: { id: "arbitrum", name: "Arbitrum", color: "#28A0F0" },
  base: { id: "base", name: "Base", color: "#0052FF" },
  optimism: { id: "optimism", name: "Optimism", color: "#FF0420" },
};

export interface DemoAsset {
  symbol: string;
  name: string;
  chain: ChainId;
  color: string;
  balance: number;
  price: number;
  change24h: number;
  costBasis: number;
  category: "l1" | "stable" | "defi" | "lst";
}

export const DEMO_ASSETS: DemoAsset[] = [
  {
    symbol: "BTC",
    name: "Bitcoin",
    chain: "bitcoin",
    color: "#F7931A",
    balance: 0.6421,
    price: 67240.12,
    change24h: 0.0321,
    costBasis: 28400,
    category: "l1",
  },
  {
    symbol: "ETH",
    name: "Ethereum",
    chain: "ethereum",
    color: "#627EEA",
    balance: 9.84,
    price: 3412.55,
    change24h: 0.0517,
    costBasis: 21500,
    category: "l1",
  },
  {
    symbol: "SOL",
    name: "Solana",
    chain: "solana",
    color: "#9945FF",
    balance: 92.3,
    price: 171.42,
    change24h: 0.0884,
    costBasis: 9100,
    category: "l1",
  },
  {
    symbol: "rETH",
    name: "Rocket Pool ETH",
    chain: "ethereum",
    color: "#00A3FF",
    balance: 2.1,
    price: 3409.8,
    change24h: 0.0509,
    costBasis: 5900,
    category: "lst",
  },
  {
    symbol: "USDC",
    name: "USD Coin",
    chain: "base",
    color: "#2775CA",
    balance: 8450.0,
    price: 1.0,
    change24h: 0.0001,
    costBasis: 8450,
    category: "stable",
  },
  {
    symbol: "ARB",
    name: "Arbitrum",
    chain: "arbitrum",
    color: "#28A0F0",
    balance: 3120,
    price: 0.92,
    change24h: -0.0412,
    costBasis: 3800,
    category: "defi",
  },
  {
    symbol: "mSOL",
    name: "Marinade SOL",
    chain: "solana",
    color: "#5FD3A6",
    balance: 14.2,
    price: 196.1,
    change24h: 0.0861,
    costBasis: 1900,
    category: "lst",
  },
  {
    symbol: "OP",
    name: "Optimism",
    chain: "optimism",
    color: "#FF0420",
    balance: 640,
    price: 2.11,
    change24h: -0.0188,
    costBasis: 1500,
    category: "defi",
  },
];

export const assetValue = (a: DemoAsset) => a.balance * a.price;

export const DEMO_TOTAL = DEMO_ASSETS.reduce((sum, a) => sum + assetValue(a), 0);

export const DEMO_CHANGE_24H =
  DEMO_ASSETS.reduce((sum, a) => sum + assetValue(a) * (a.change24h / (1 + a.change24h)), 0) /
  DEMO_TOTAL;

export const DEMO_COST_BASIS = DEMO_ASSETS.reduce((sum, a) => sum + a.costBasis, 0);

export type Range = "1D" | "1W" | "1M" | "3M" | "1Y" | "ALL";
export const RANGES: Range[] = ["1D", "1W", "1M", "3M", "1Y", "ALL"];

const RANGE_SPEC: Record<Range, { points: number; stepMs: number; drift: number; vol: number }> = {
  "1D": { points: 96, stepMs: 15 * 60_000, drift: 0.0003, vol: 0.003 },
  "1W": { points: 84, stepMs: 2 * 3_600_000, drift: 0.0006, vol: 0.006 },
  "1M": { points: 90, stepMs: 8 * 3_600_000, drift: 0.0009, vol: 0.01 },
  "3M": { points: 90, stepMs: 86_400_000, drift: 0.002, vol: 0.02 },
  "1Y": { points: 120, stepMs: 3 * 86_400_000, drift: 0.004, vol: 0.03 },
  ALL: { points: 140, stepMs: 7 * 86_400_000, drift: 0.007, vol: 0.04 },
};

/** Small deterministic PRNG so the demo chart looks the same on every launch. */
function mulberry32(seed: number) {
  let a = seed;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface HistoryPoint {
  t: number;
  value: number;
}

/** Random walk generated backwards from the current total, so it always ends at "now". */
export function demoHistory(range: Range, end = DEMO_TOTAL, now = Date.now()): HistoryPoint[] {
  const { points, stepMs, drift, vol } = RANGE_SPEC[range];
  const rand = mulberry32(range.charCodeAt(0) * 131 + range.length * 7);
  const values = [end];
  for (let i = 1; i < points; i++) {
    const prev = values[i - 1] ?? end;
    const shock = (rand() - 0.5) * 2 * vol + Math.sin(i / 9) * vol * 0.4;
    values.push(prev / (1 + drift + shock));
  }
  return values.reverse().map((value, i) => ({ t: now - (points - 1 - i) * stepMs, value }));
}

/** 24 points for the small per-asset sparklines. */
export function demoSparkline(asset: DemoAsset): number[] {
  const rand = mulberry32(asset.symbol.charCodeAt(0) * 977 + asset.symbol.length);
  const out = [asset.price];
  for (let i = 1; i < 24; i++) {
    const prev = out[i - 1] ?? asset.price;
    out.push(prev / (1 + asset.change24h / 24 + (rand() - 0.5) * 0.012));
  }
  return out.reverse();
}

export type ActivityKind = "received" | "sent" | "swap" | "staked";

export interface DemoActivity {
  kind: ActivityKind;
  label: string;
  chain: ChainId;
  amount: string;
  value: number;
  minutesAgo: number;
}

export const DEMO_ACTIVITY: DemoActivity[] = [
  {
    kind: "received",
    label: "ETH",
    chain: "ethereum",
    amount: "+0.42 ETH",
    value: 1433.27,
    minutesAgo: 18,
  },
  {
    kind: "swap",
    label: "USDC → SOL",
    chain: "solana",
    amount: "+12.5 SOL",
    value: 2142.75,
    minutesAgo: 140,
  },
  {
    kind: "staked",
    label: "mSOL",
    chain: "solana",
    amount: "+4.0 mSOL",
    value: 784.4,
    minutesAgo: 1260,
  },
  {
    kind: "sent",
    label: "USDC",
    chain: "base",
    amount: "-500 USDC",
    value: -500,
    minutesAgo: 2880,
  },
];

export const DEMO_RISK = {
  score: 34,
  factors: [
    { key: "concentration", value: 0.48 },
    { key: "stablecoins", value: 0.12 },
    { key: "approvals", value: 0.3 },
  ],
} as const;
