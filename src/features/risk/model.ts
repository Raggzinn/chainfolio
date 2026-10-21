import type { RiskFactorKey, RiskReport } from "@/lib/api";

export type RiskLevel = "riskLow" | "riskModerate" | "riskHigh";

export function riskLevel(score: number): RiskLevel {
  if (score < 35) return "riskLow";
  if (score < 65) return "riskModerate";
  return "riskHigh";
}

export const LEVEL_COLOR: Record<RiskLevel, string> = {
  riskLow: "var(--success)",
  riskModerate: "var(--warning)",
  riskHigh: "var(--danger)",
};

/** Factor scores as 0–1 bars, in the backend's order. */
export function factorBars(report: RiskReport): { key: RiskFactorKey; value: number }[] {
  return report.factors.map((f) => ({ key: f.key, value: f.score / 100 }));
}

/** Allowance shown in whole tokens, or `null` for "unlimited". */
export function approvalAmount(amount: string | null, unlimited: boolean): number | null {
  if (unlimited || amount === null) return null;
  return Number(amount);
}
