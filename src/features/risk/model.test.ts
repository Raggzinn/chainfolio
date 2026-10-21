import type { RiskReport } from "@/lib/api";
import { approvalAmount, factorBars, riskLevel } from "./model";

describe("risk model", () => {
  it("maps scores to levels", () => {
    expect(riskLevel(0)).toBe("riskLow");
    expect(riskLevel(34)).toBe("riskLow");
    expect(riskLevel(35)).toBe("riskModerate");
    expect(riskLevel(64)).toBe("riskModerate");
    expect(riskLevel(65)).toBe("riskHigh");
  });

  it("turns factor scores into bars", () => {
    const report = {
      factors: [
        { key: "concentration", score: 48, weight: "0.3" },
        { key: "approvals", score: 100, weight: "0.2" },
      ],
    } as RiskReport;
    expect(factorBars(report)).toEqual([
      { key: "concentration", value: 0.48 },
      { key: "approvals", value: 1 },
    ]);
  });

  it("formats allowances", () => {
    expect(approvalAmount(null, true)).toBeNull();
    expect(approvalAmount("2500", false)).toBe(2500);
    expect(approvalAmount(null, false)).toBeNull();
  });
});
