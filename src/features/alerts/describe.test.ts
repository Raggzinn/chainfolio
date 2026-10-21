import i18n from "@/i18n";
import type { AlertEvent } from "@/lib/api";
import { type Describer, describeCondition, describeEvent, formatUsd } from "./describe";
import { EMPTY, percentToRatio, toCondition } from "./rule-dialog";

const describer = (hidden = false): Describer => ({
  t: i18n.getFixedT("en"),
  locale: "en",
  wallet: (id) => (id === null ? "any wallet" : `Wallet ${id}`),
  hidden,
});

describe("alert descriptions", () => {
  it("keeps four decimals under a dollar", () => {
    expect(formatUsd(0.97512, "en")).toBe("$0.9751");
    expect(formatUsd(101234, "en")).toBe("$101,234.00");
  });

  it("describes rules", () => {
    const d = describer();
    expect(
      describeCondition(
        { kind: "priceAbove", priceKey: "coingecko:bitcoin", symbol: "BTC", price: "100000" },
        d,
      ),
    ).toBe("BTC goes above $100,000.00");
    expect(describeCondition({ kind: "portfolioChange", direction: "down", ratio: "0.1" }, d)).toBe(
      "Portfolio falls 10% in 24h",
    );
    expect(describeCondition({ kind: "depeg", ratio: "0.01" }, d)).toBe(
      "A stablecoin you hold drifts 1% from $1",
    );
  });

  it("masks wallet amounts in privacy mode but not prices", () => {
    const event: AlertEvent = {
      id: 1,
      ruleId: 1,
      condition: { kind: "balanceChange", walletId: null, direction: "either", usd: "1000" },
      value: "-2500",
      subject: "3",
      triggeredAt: 0,
    };
    expect(describeEvent(event, describer())).toBe("$2,500.00 left Wallet 3");
    expect(describeEvent(event, describer(true))).not.toContain("2,500");
    expect(
      describeEvent(
        { ...event, condition: { kind: "depeg", ratio: "0.01" }, value: "0.975", subject: "USDT" },
        describer(true),
      ),
    ).toBe("USDT traded at $0.975");
  });
});

describe("alert form", () => {
  const symbolOf = (key: string) => (key === "coingecko:bitcoin" ? "BTC" : undefined);

  it("converts percentages to ratios", () => {
    expect(percentToRatio("5")).toBe("0.05");
    expect(percentToRatio("2,5")).toBe("0.025");
  });

  it("builds conditions only from complete, sane input", () => {
    expect(toCondition({ ...EMPTY, priceKey: "coingecko:bitcoin", amount: "" }, symbolOf)).toBe(
      null,
    );
    expect(toCondition({ ...EMPTY, priceKey: "coingecko:bitcoin", amount: "-1" }, symbolOf)).toBe(
      null,
    );
    expect(
      toCondition({ ...EMPTY, priceKey: "coingecko:bitcoin", amount: "100000" }, symbolOf),
    ).toEqual({
      kind: "priceAbove",
      priceKey: "coingecko:bitcoin",
      symbol: "BTC",
      price: "100000",
    });
    expect(toCondition({ ...EMPTY, kind: "depeg", percent: "150" }, symbolOf)).toBe(null);
    expect(
      toCondition(
        { ...EMPTY, kind: "balanceChange", wallet: "4", direction: "up", amount: "500" },
        symbolOf,
      ),
    ).toEqual({ kind: "balanceChange", walletId: 4, direction: "up", usd: "500" });
  });
});
