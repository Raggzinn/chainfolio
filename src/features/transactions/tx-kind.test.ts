import type { TxItem, TxLeg } from "@/lib/api";
import { headlineValue, legSymbols } from "./tx-kind";

const leg = (symbol: string, amount: string, valueUsd: string | null): TxLeg => ({
  asset: { id: symbol, chain: "ethereum", symbol, name: symbol, logoUrl: null },
  amount,
  valueUsd,
  counterparty: null,
});

const tx = (legs: TxLeg[]): TxItem => ({
  id: "ethereum:0x1",
  chain: "ethereum",
  hash: "0x1",
  timestamp: 0,
  walletId: 1,
  walletLabel: "Main",
  family: "evm",
  kind: "swap",
  autoKind: "swap",
  note: null,
  failed: false,
  legs,
  fee: null,
});

describe("transaction helpers", () => {
  it("splits symbols by direction", () => {
    const swap = tx([leg("USDC", "-3000", "-3000"), leg("ETH", "1", "3010")]);
    expect(legSymbols(swap)).toEqual({ out: "USDC", in: "ETH" });
  });

  it("prefers the value received", () => {
    const swap = tx([leg("USDC", "-3000", "-3000"), leg("ETH", "1", "3010")]);
    expect(headlineValue(swap)).toBe(3010);
  });

  it("falls back to the value sent", () => {
    expect(headlineValue(tx([leg("ETH", "-1", "-2500")]))).toBe(2500);
  });

  it("is unknown without prices", () => {
    expect(headlineValue(tx([leg("FOO", "5", null)]))).toBeNull();
  });
});
