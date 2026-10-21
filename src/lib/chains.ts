import type { ChainId, Family } from "./api";

export interface ChainInfo {
  id: ChainId;
  name: string;
  color: string;
}

/** Display metadata for every chain the backend scans. */
export const CHAINS: Record<ChainId, ChainInfo> = {
  ethereum: { id: "ethereum", name: "Ethereum", color: "#627EEA" },
  bitcoin: { id: "bitcoin", name: "Bitcoin", color: "#F7931A" },
  solana: { id: "solana", name: "Solana", color: "#14F195" },
  arbitrum: { id: "arbitrum", name: "Arbitrum", color: "#28A0F0" },
  base: { id: "base", name: "Base", color: "#0052FF" },
  optimism: { id: "optimism", name: "Optimism", color: "#FF0420" },
  polygon: { id: "polygon", name: "Polygon", color: "#8247E5" },
  bsc: { id: "bsc", name: "BNB Chain", color: "#F3BA2F" },
  avalanche: { id: "avalanche", name: "Avalanche", color: "#E84142" },
  tron: { id: "tron", name: "TRON", color: "#FF060A" },
  xrp: { id: "xrp", name: "XRP Ledger", color: "#7B8794" },
};

export const CHAIN_IDS = Object.keys(CHAINS) as ChainId[];

export const FAMILY_NAMES: Record<Family, string> = {
  evm: "EVM",
  solana: "Solana",
  bitcoin: "Bitcoin",
  tron: "TRON",
  xrpl: "XRP Ledger",
};

/** Native coin shown as the icon of a wallet of each family. */
export const FAMILY_SYMBOL: Record<Family, string> = {
  evm: "ETH",
  solana: "SOL",
  bitcoin: "BTC",
  tron: "TRX",
  xrpl: "XRP",
};

/** Network icon shown for a wallet of each family. */
export const FAMILY_ICON: Record<Family, ChainId> = {
  evm: "ethereum",
  solana: "solana",
  bitcoin: "bitcoin",
  tron: "tron",
  xrpl: "xrp",
};

/** Brand colors for common tickers; anything else falls back to its chain. */
const TOKEN_COLORS: Record<string, string> = {
  BTC: "#F7931A",
  WBTC: "#F7931A",
  CBBTC: "#F7931A",
  ETH: "#627EEA",
  WETH: "#627EEA",
  STETH: "#00A3FF",
  WSTETH: "#00A3FF",
  RETH: "#00A3FF",
  SOL: "#9945FF",
  MSOL: "#5FD3A6",
  JITOSOL: "#5FD3A6",
  USDC: "#2775CA",
  USDT: "#26A17B",
  DAI: "#F5AC37",
  ARB: "#28A0F0",
  OP: "#FF0420",
  POL: "#8247E5",
  BNB: "#F3BA2F",
  AVAX: "#E84142",
  TRX: "#FF060A",
  XRP: "#7B8794",
  LINK: "#2A5ADA",
  UNI: "#FF007A",
};

export function tokenColor(symbol: string, chain: ChainId): string {
  return TOKEN_COLORS[symbol.toUpperCase()] ?? CHAINS[chain].color;
}

/** `0x1234…abcd` style shortening for long addresses and keys. */
export function shortAddress(address: string, head = 6, tail = 4): string {
  return address.length <= head + tail + 1
    ? address
    : `${address.slice(0, head)}…${address.slice(-tail)}`;
}
