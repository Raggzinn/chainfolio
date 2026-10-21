import type { Position } from "@/lib/api";
import { groupByProtocol, healthLevel, protocolInfo, summarize } from "./model";

function position(protocol: string, value: string | null, extra: Partial<Position> = {}): Position {
  return {
    walletId: 1,
    chain: "ethereum",
    protocol,
    kind: "lending",
    name: protocol,
    tokens: [],
    valueUsd: value,
    apy: null,
    healthFactor: null,
    inBalances: false,
    receipts: [],
    impermanentLoss: null,
    ...extra,
  };
}

describe("defi model", () => {
  it("groups positions by protocol, largest first", () => {
    const groups = groupByProtocol([
      position("lido", "500"),
      position("aave-v3", "300"),
      position("aave-v3", "900"),
      position("uniswap-v3", null),
    ]);
    expect(groups.map((g) => g.protocol)).toEqual(["aave-v3", "lido", "uniswap-v3"]);
    const [aave] = groups;
    expect(aave?.value).toBe(1200);
    expect(aave?.positions.map((p) => p.valueUsd)).toEqual(["900", "300"]);
    expect(aave?.info.name).toBe("Aave v3");
  });

  it("summarizes value, yield and health", () => {
    const summary = summarize([
      position("aave-v3", "1000", { apy: "0.05", healthFactor: "1.8" }),
      position("lido", "3000", { apy: "0.03" }),
      position("uniswap-v3", "1000"),
      position("compound-v3", "500", { healthFactor: "3.2" }),
    ]);
    expect(summary.total).toBe(5500);
    expect(summary.yearlyYield).toBeCloseTo(140);
    expect(summary.apy).toBeCloseTo(0.035);
    expect(summary.minHealthFactor).toBe(1.8);
    expect(summarize([]).apy).toBeNull();
  });

  it("classifies health factors", () => {
    expect(healthLevel(3)).toBe("safe");
    expect(healthLevel(1.5)).toBe("watch");
    expect(healthLevel(1.1)).toBe("danger");
  });

  it("falls back for unknown protocols", () => {
    expect(protocolInfo("mystery").name).toBe("mystery");
  });
});
