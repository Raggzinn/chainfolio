import { DEMO_TOTAL } from "@/features/demo/data";
import type { Portfolio } from "@/lib/api";
import { demoAssetRows, liveAssetRows, rowsTotal } from "./model";

const portfolio: Portfolio = {
  totalUsd: "4100",
  change24hUsd: "800",
  change24h: "0.24242424",
  assets: [
    {
      id: "base:0xusdc",
      chain: "base",
      symbol: "USDC",
      name: "USD Coin",
      logoUrl: null,
      amount: "100",
      locked: null,
      priceUsd: "1",
      valueUsd: "100",
      change24h: null,
    },
    {
      id: "ethereum:native",
      chain: "ethereum",
      symbol: "ETH",
      name: "Ether",
      logoUrl: null,
      amount: "2",
      locked: null,
      priceUsd: "2000",
      valueUsd: "4000",
      change24h: "0.25",
    },
    {
      id: "polygon:0xabc",
      chain: "polygon",
      symbol: "XYZ",
      name: "Some token",
      logoUrl: "https://example.com/xyz.png",
      amount: "1",
      locked: "0.2",
      priceUsd: "0.5",
      valueUsd: "0.5",
      change24h: "-0.1",
    },
  ],
  chains: [],
  wallets: [],
  unpricedCount: 0,
  defiUsd: "0",
  spamCount: 0,
  syncedAt: null,
};

describe("liveAssetRows", () => {
  it("converts decimal strings and sorts by value", () => {
    const rows = liveAssetRows(portfolio);
    expect(rows.map((r) => r.key)).toEqual(["ethereum:native", "base:0xusdc", "polygon:0xabc"]);
    expect(rows[0]).toMatchObject({ balance: 2, price: 2000, value: 4000, change24h: 0.25 });
    expect(rows[1]?.change24h).toBeNull();
    expect(rowsTotal(rows)).toBeCloseTo(4100.5);
  });

  it("uses brand colors for known tickers and the chain color otherwise", () => {
    const rows = liveAssetRows(portfolio);
    expect(rows.find((r) => r.symbol === "USDC")?.color).toBe("#2775CA");
    expect(rows.find((r) => r.symbol === "XYZ")?.color).toBe("#8247E5");
    expect(rows.find((r) => r.symbol === "XYZ")?.logoUrl).toBe("https://example.com/xyz.png");
    expect(rows.find((r) => r.symbol === "XYZ")?.locked).toBe(0.2);
    expect(rows.find((r) => r.symbol === "USDC")?.locked).toBeUndefined();
  });
});

describe("demoAssetRows", () => {
  it("adds up to the demo total", () => {
    expect(rowsTotal(demoAssetRows())).toBeCloseTo(DEMO_TOTAL);
  });
});
