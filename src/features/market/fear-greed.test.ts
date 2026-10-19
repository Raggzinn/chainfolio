import { moodLevel, parseMarketMood } from "./fear-greed";

describe("moodLevel", () => {
  it.each([
    [0, "extremeFear"],
    [24, "extremeFear"],
    [25, "fear"],
    [46, "fear"],
    [47, "neutral"],
    [54, "neutral"],
    [55, "greed"],
    [74, "greed"],
    [75, "extremeGreed"],
    [100, "extremeGreed"],
  ])("%i -> %s", (value, level) => {
    expect(moodLevel(value)).toBe(level);
  });
});

describe("parseMarketMood", () => {
  const day = 86_400;
  const rows = Array.from({ length: 31 }, (_, i) => ({
    value: String(60 - i),
    timestamp: String(1_792_368_000 - i * day),
    time_until_update: i === 0 ? "3600" : undefined,
  }));

  it("maps today, yesterday, last week and last month", () => {
    const mood = parseMarketMood({ data: rows });
    expect(mood.today).toEqual({ value: 60, level: "greed", date: 1_792_368_000_000 });
    expect(mood.yesterday?.value).toBe(59);
    expect(mood.lastWeek?.value).toBe(53);
    expect(mood.lastMonth?.value).toBe(30);
    expect(mood.nextUpdateIn).toBe(3_600_000);
  });

  it("orders history oldest to newest", () => {
    const { history } = parseMarketMood({ data: rows });
    expect(history[0]?.value).toBe(30);
    expect(history.at(-1)?.value).toBe(60);
  });

  it("clamps out-of-range values", () => {
    const mood = parseMarketMood({ data: [{ value: "140", timestamp: "1" }] });
    expect(mood.today.value).toBe(100);
  });

  it("throws on an empty payload", () => {
    expect(() => parseMarketMood({ data: [] })).toThrow();
  });
});
