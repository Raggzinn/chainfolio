import { DEMO_TOTAL, demoHistory, RANGES } from "./data";

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
