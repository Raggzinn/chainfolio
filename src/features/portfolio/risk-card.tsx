import { Link } from "@tanstack/react-router";
import { ChevronRight } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { Skeleton } from "@/components/ui/skeleton";
import { DEMO_RISK } from "@/features/demo/data";
import { factorBars, LEVEL_COLOR, riskLevel } from "@/features/risk/model";
import { useAutoScan, useIsScanning, useRiskReport } from "@/features/risk/queries";
import { gsap, prefersReducedMotion, useGSAP } from "@/lib/gsap";
import { cn } from "@/lib/utils";

const SIZE = 148;
const STROKE = 12;

export interface GaugeFactor {
  key: string;
  label: string;
  /** 0–1 */
  value: number;
}

/** Animated 0–100 risk gauge with a bar per factor. */
export function RiskGauge({
  score,
  factors,
  size = SIZE,
  className,
}: {
  score: number;
  factors: GaugeFactor[];
  size?: number;
  className?: string;
}) {
  const { t } = useTranslation();
  const scope = useRef<HTMLDivElement>(null);
  const scoreRef = useRef<HTMLSpanElement>(null);
  const level = riskLevel(score);
  const r = (size - STROKE) / 2;
  const circ = 2 * Math.PI * r;
  // The gauge is a 270° arc open at the bottom.
  const arc = circ * 0.75;

  useGSAP(
    () => {
      const counter = { n: 0 };
      const instant = prefersReducedMotion();
      gsap.fromTo(
        "[data-gauge]",
        { strokeDashoffset: arc },
        {
          strokeDashoffset: arc * (1 - score / 100),
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
    { scope, dependencies: [score, factors.length], revertOnUpdate: true },
  );

  return (
    <div ref={scope} className={cn("flex flex-col items-center gap-5", className)}>
      <div className="relative" style={{ width: size, height: size }}>
        <svg width={size} height={size} className="rotate-[135deg]" aria-hidden="true">
          <circle
            cx={size / 2}
            cy={size / 2}
            r={r}
            fill="none"
            stroke="currentColor"
            className="text-muted"
            strokeWidth={STROKE}
            strokeLinecap="round"
            strokeDasharray={`${arc} ${circ}`}
          />
          <circle
            data-gauge
            cx={size / 2}
            cy={size / 2}
            r={r}
            fill="none"
            stroke={LEVEL_COLOR[level]}
            strokeWidth={STROKE}
            strokeLinecap="round"
            strokeDasharray={`${arc} ${circ}`}
            strokeDashoffset={arc}
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
              <span className="text-muted-foreground">{f.label}</span>
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
  );
}

/** Demo-mode card with sample factors. */
export function RiskCard() {
  const { t } = useTranslation();
  return (
    <Panel tourId="risk" title={t("dashboard.risk")} subtitle={t("dashboard.riskHint")}>
      <RiskGauge
        score={DEMO_RISK.score}
        factors={DEMO_RISK.factors.map((f) => ({
          key: f.key,
          label: t(`dashboard.factors.${f.key}`),
          value: f.value,
        }))}
      />
    </Panel>
  );
}

/** Real score from the local risk report; scans in the background when stale. */
export function LiveRiskCard() {
  const { t } = useTranslation();
  const { data } = useRiskReport();
  const scanning = useIsScanning();
  useAutoScan(true);

  return (
    <Panel
      tourId="risk"
      title={t("dashboard.risk")}
      subtitle={scanning ? t("risk.scanning") : t("dashboard.riskHint")}
      action={
        <Link
          to="/risk"
          className="group flex items-center gap-0.5 font-medium text-primary text-xs"
        >
          {t("dashboard.viewAll")}
          <ChevronRight className="size-3.5 transition-transform group-hover:translate-x-0.5" />
        </Link>
      }
    >
      {data ? (
        <RiskGauge
          score={data.score}
          factors={factorBars(data).map((f) => ({
            ...f,
            label: t(`risk.factors.${f.key}.title`),
          }))}
        />
      ) : (
        <div className="flex flex-col items-center gap-5">
          <Skeleton className="rounded-full" style={{ width: SIZE, height: SIZE }} />
          <Skeleton className="h-24 w-full rounded-xl" />
        </div>
      )}
    </Panel>
  );
}
