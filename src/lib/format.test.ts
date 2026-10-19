import { formatCurrency, formatPercent, MASK } from "./format";

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
