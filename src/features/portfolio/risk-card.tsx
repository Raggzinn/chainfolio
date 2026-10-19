import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { DEMO_RISK } from "@/features/demo/data";
import { gsap, prefersReducedMotion, useGSAP } from "@/lib/gsap";

const SIZE = 148;
const STROKE = 12;
const R = (SIZE - STROKE) / 2;
const CIRC = 2 * Math.PI * R;
// The gauge is a 270° arc open at the bottom.
const ARC = CIRC * 0.75;

const LEVEL_COLOR = {
  riskLow: "var(--success)",
  riskModerate: "var(--warning)",
  riskHigh: "var(--danger)",
} as const;

export function RiskCard() {
  const { t } = useTranslation();
  const scope = useRef<HTMLDivElement>(null);
  const scoreRef = useRef<HTMLSpanElement>(null);
  const { score, factors } = DEMO_RISK;
  const level: keyof typeof LEVEL_COLOR =
    score < 35 ? "riskLow" : score < 65 ? "riskModerate" : "riskHigh";

  useGSAP(
    () => {
      const counter = { n: 0 };
      const instant = prefersReducedMotion();
      gsap.fromTo(
        "[data-gauge]",
        { strokeDashoffset: ARC },
        {
          strokeDashoffset: ARC * (1 - score / 100),
          duration: instant ? 0 : 1.6,
          ease: "expo.out",
          delay: 0.2,
        },
      );
      gsap.to(counter, {
        n: score,
        duration: instant ? 0 : 1.6,
        ease: "expo.out",
        delay: 0.2,
        onUpdate: () => {
          if (scoreRef.current) scoreRef.current.textContent = Math.round(counter.n).toString();
        },
      });
      gsap.from("[data-factor]", {
        scaleX: 0,
        transformOrigin: "left center",
        duration: instant ? 0 : 1.1,
        ease: "expo.out",
        stagger: 0.12,
        delay: 0.4,
      });
    },
    { scope },
  );

  return (
    <Panel title={t("dashboard.risk")} subtitle={t("dashboard.riskHint")}>
      <div ref={scope} className="flex flex-col items-center gap-5">
        <div className="relative" style={{ width: SIZE, height: SIZE }}>
          <svg width={SIZE} height={SIZE} className="rotate-[135deg]" aria-hidden="true">
            <circle
              cx={SIZE / 2}
              cy={SIZE / 2}
              r={R}
              fill="none"
              stroke="currentColor"
              className="text-muted"
              strokeWidth={STROKE}
              strokeLinecap="round"
              strokeDasharray={`${ARC} ${CIRC}`}
            />
            <circle
              data-gauge
              cx={SIZE / 2}
              cy={SIZE / 2}
              r={R}
              fill="none"
              stroke={LEVEL_COLOR[level]}
              strokeWidth={STROKE}
              strokeLinecap="round"
              strokeDasharray={`${ARC} ${CIRC}`}
              strokeDashoffset={ARC}
            />
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <span ref={scoreRef} className="tabular font-semibold text-4xl tracking-tight">
              0
            </span>
            <span className="font-medium text-xs" style={{ color: LEVEL_COLOR[level] }}>
              {t(`dashboard.${level}`)}
            </span>
          </div>
        </div>

        <ul className="w-full space-y-3">
          {factors.map((f) => (
            <li key={f.key} className="space-y-1.5">
              <div className="flex justify-between text-xs">
                <span className="text-muted-foreground">{t(`dashboard.factors.${f.key}`)}</span>
                <span className="tabular font-medium">{Math.round(f.value * 100)}</span>
              </div>
              <div className="h-1.5 overflow-hidden rounded-full bg-muted">
                <div
                  data-factor
                  className="h-full rounded-full bg-primary"
                  style={{ width: `${f.value * 100}%` }}
                />
              </div>
            </li>
          ))}
        </ul>
      </div>
    </Panel>
  );
}
