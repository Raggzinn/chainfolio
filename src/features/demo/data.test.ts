import {
  assetValue,
  DEMO_ASSETS,
  DEMO_HISTORY_DAYS,
  DEMO_TOTAL,
  demoHistory,
  demoPortfolioHistory,
  demoRangeHistory,
  RANGES,
} from "./data";

const DAY = 86_400_000;
const NOW = 1_800_000_000_000;

describe("demoHistory", () => {
  it.each(RANGES)("ends at the current total for %s", (range) => {
    const series = demoHistory(range, DEMO_TOTAL, 0);
    expect(series.at(-1)?.value).toBeCloseTo(DEMO_TOTAL);
    expect(series.at(-1)?.t).toBe(0);
  });

  it("is deterministic", () => {
    expect(demoHistory("1M", 100, 0)).toEqual(demoHistory("1M", 100, 0));
  });

  it("is ordered by time", () => {
    const series = demoHistory("1Y", 100, 0);
    for (let i = 1; i < series.length; i++) {
      expect(series[i]?.t).toBeGreaterThan(series[i - 1]?.t ?? Number.NEGATIVE_INFINITY);
    }
  });
});

describe("demoPortfolioHistory", () => {
  it("ends at the full portfolio value without a filter", () => {
    expect(demoPortfolioHistory("1M", undefined, NOW).at(-1)?.value).toBeCloseTo(DEMO_TOTAL);
  });

  it("only sums the selected assets", () => {
    const [btc, eth] = DEMO_ASSETS;
    if (!btc || !eth) throw new Error("demo assets missing");
    const series = demoPortfolioHistory("1W", [btc.symbol, eth.symbol], NOW);
    expect(series.at(-1)?.value).toBeCloseTo(assetValue(btc) + assetValue(eth));
  });

  it("keeps stablecoins nearly flat", () => {
    const series = demoPortfolioHistory("1Y", ["USDC"], NOW).map((p) => p.value);
    expect(Math.max(...series) / Math.min(...series)).toBeLessThan(1.1);
  });
});

describe("demoRangeHistory", () => {
  it("returns one point per day, inclusive", () => {
    const series = demoRangeHistory(new Date(NOW - 9 * DAY), new Date(NOW), undefined, NOW);
    expect(series).toHaveLength(10);
  });

  it("ends at today's value when the range ends today", () => {
    const series = demoRangeHistory(new Date(NOW - 30 * DAY), new Date(NOW), undefined, NOW);
    expect(series.at(-1)?.value).toBeCloseTo(DEMO_TOTAL);
  });

  it("clamps ranges older than the demo history", () => {
    const series = demoRangeHistory(new Date(0), new Date(NOW), undefined, NOW);
    expect(series).toHaveLength(DEMO_HISTORY_DAYS);
  });
});
