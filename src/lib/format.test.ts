import { formatCurrency, formatPercent, formatRelative, MASK } from "./format";

describe("formatCurrency", () => {
  it("formats USD in en", () => {
    expect(formatCurrency(124532.18, "USD", "en")).toBe("$124,532.18");
  });

  it("formats BRL in pt-BR", () => {
    expect(formatCurrency(1234.5, "BRL", "pt-BR").replace(/\s/g, " ")).toBe("R$ 1.234,50");
  });

  it("masks values in privacy mode", () => {
    expect(formatCurrency(1, "USD", "en", { hidden: true })).toBe(MASK);
  });
});

describe("formatPercent", () => {
  it("adds a sign to positive changes", () => {
    expect(formatPercent(0.124, "en")).toBe("+12.4%");
  });

  it("keeps the minus sign on losses", () => {
    expect(formatPercent(-0.05, "en")).toBe("-5%");
  });
});

describe("formatRelative", () => {
  const now = 1_800_000_000_000;

  it("picks the largest whole unit", () => {
    expect(formatRelative(now - 5 * 60_000, "en", now)).toBe("5 minutes ago");
    expect(formatRelative(now - 3 * 3_600_000, "en", now)).toBe("3 hours ago");
    expect(formatRelative(now - 2 * 86_400_000, "pt-BR", now)).toBe("anteontem");
  });

  it("says now for recent or future timestamps", () => {
    expect(formatRelative(now - 10_000, "en", now)).toBe("now");
    expect(formatRelative(now + 60_000, "en", now)).toBe("now");
  });
});
