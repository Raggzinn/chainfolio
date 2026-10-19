import { RefreshCw } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { Skeleton } from "@/components/ui/skeleton";
import { gsap, prefersReducedMotion, useGSAP } from "@/lib/gsap";
import { cn } from "@/lib/utils";
import {
  MOOD_COLORS,
  type MoodLevel,
  type MoodPoint,
  moodLevel,
  useMarketMood,
} from "./fear-greed";

const W = 240;
const CX = W / 2;
const CY = 118;
const R = 96;
const STROKE = 14;

const BANDS: { from: number; to: number; level: MoodLevel }[] = [
  { from: 0, to: 25, level: "extremeFear" },
  { from: 25, to: 47, level: "fear" },
  { from: 47, to: 55, level: "neutral" },
  { from: 55, to: 75, level: "greed" },
  { from: 75, to: 100, level: "extremeGreed" },
];

/** Point on the gauge arc for a 0–100 value (0 = left, 100 = right). */
function point(value: number, radius = R) {
  const angle = Math.PI - (value / 100) * Math.PI;
  return { x: CX + radius * Math.cos(angle), y: CY - radius * Math.sin(angle) };
}

function arc(from: number, to: number) {
  const a = point(from);
  const b = point(to);
  return `M${a.x.toFixed(2)} ${a.y.toFixed(2)} A${R} ${R} 0 0 1 ${b.x.toFixed(2)} ${b.y.toFixed(2)}`;
}

export function MarketMoodCard() {
  const { t } = useTranslation();
  const { data, isPending, isError, refetch, isFetching } = useMarketMood();

  const hours = data?.nextUpdateIn ? Math.max(1, Math.round(data.nextUpdateIn / 3_600_000)) : null;

  return (
    <Panel
      title={t("market.title")}
      subtitle={
        hours
          ? `${t("market.index")} · ${t("market.updatesIn", { count: hours })}`
          : t("market.index")
      }
    >
      {isPending ? (
        <MoodSkeleton />
      ) : isError ? (
        <div className="flex flex-col items-center gap-3 py-8 text-center">
          <p className="text-muted-foreground text-sm">{t("market.unavailable")}</p>
          <button
            type="button"
            onClick={() => refetch()}
            className="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 font-medium text-primary-foreground text-xs transition-colors hover:bg-primary/90"
          >
            <RefreshCw className={cn("size-3.5", isFetching && "animate-spin")} />
            {t("market.retry")}
          </button>
        </div>
      ) : (
        <MoodContent
          today={data.today}
          comparisons={[
            { key: "yesterday", point: data.yesterday },
            { key: "lastWeek", point: data.lastWeek },
            { key: "lastMonth", point: data.lastMonth },
          ]}
          history={data.history}
        />
      )}
    </Panel>
  );
}

function MoodContent({
  today,
  comparisons,
  history,
}: {
  today: MoodPoint;
  comparisons: { key: "yesterday" | "lastWeek" | "lastMonth"; point?: MoodPoint }[];
  history: MoodPoint[];
}) {
  const { t, i18n } = useTranslation();
  const scope = useRef<HTMLDivElement>(null);
  const needle = useRef<SVGGElement>(null);
  const valueRef = useRef<HTMLSpanElement>(null);
  const color = MOOD_COLORS[today.level];
  const dayFormat = new Intl.DateTimeFormat(i18n.resolvedLanguage, {
    day: "numeric",
    month: "short",
  });

  useGSAP(
    () => {
      const instant = prefersReducedMotion();
      const target = -90 + today.value * 1.8;
      const counter = { n: 0 };

      gsap.fromTo(
        needle.current,
        { rotation: -90, svgOrigin: `${CX} ${CY}` },
        {
          rotation: target,
          svgOrigin: `${CX} ${CY}`,
          duration: instant ? 0 : 1.8,
          ease: "elastic.out(1, 0.55)",
          delay: 0.25,
        },
      );
      gsap.to(counter, {
        n: today.value,
        duration: instant ? 0 : 1.4,
        ease: "expo.out",
        delay: 0.25,
        onUpdate: () => {
          if (valueRef.current) valueRef.current.textContent = Math.round(counter.n).toString();
        },
      });
      gsap.from("[data-band]", {
        autoAlpha: 0,
        duration: instant ? 0 : 0.5,
        stagger: 0.07,
        ease: "power2.out",
      });
      gsap.from("[data-bar]", {
        scaleY: 0,
        transformOrigin: "bottom center",
        duration: instant ? 0 : 0.7,
        stagger: 0.018,
        ease: "expo.out",
        delay: 0.35,
      });
    },
    { scope, dependencies: [today.value] },
  );

  return (
    <div ref={scope} className="space-y-5">
      <div className="relative mx-auto" style={{ width: W }}>
        <svg viewBox={`0 0 ${W} ${CY + 14}`} className="w-full overflow-visible" aria-hidden="true">
          {BANDS.map((band) => (
            <path
              key={band.level}
              data-band
              d={arc(band.from + 0.9, band.to - 0.9)}
              fill="none"
              stroke={MOOD_COLORS[band.level]}
              strokeWidth={STROKE}
              strokeLinecap="round"
              opacity={band.level === today.level ? 1 : 0.22}
            />
          ))}
          <g ref={needle} transform={`rotate(-90 ${CX} ${CY})`}>
            <line
              x1={CX}
              y1={CY}
              x2={CX}
              y2={CY - R + STROKE + 4}
              stroke="var(--foreground)"
              strokeWidth={3}
              strokeLinecap="round"
            />
          </g>
          <circle
            cx={CX}
            cy={CY}
            r={9}
            fill="var(--card)"
            stroke="var(--foreground)"
            strokeWidth={3}
          />
        </svg>
      </div>

      <div className="-mt-2 flex flex-col items-center gap-1">
        <span
          ref={valueRef}
          className="tabular font-semibold text-[40px] leading-none tracking-tight"
        >
          0
        </span>
        <span className="font-semibold text-sm" style={{ color }}>
          {t(`market.levels.${today.level}`)}
        </span>
      </div>

      <dl className="grid grid-cols-3 gap-2">
        {comparisons.map(({ key, point }) => (
          <div key={key} className="rounded-xl bg-muted px-3 py-2.5">
            <dt className="truncate text-[11px] text-muted-foreground">{t(`market.${key}`)}</dt>
            <dd className="mt-1 flex items-center gap-1.5">
              <span
                className="size-2 shrink-0 rounded-full"
                style={{ background: point ? MOOD_COLORS[point.level] : "var(--border)" }}
              />
              <span className="tabular font-semibold text-sm">{point?.value ?? "—"}</span>
            </dd>
          </div>
        ))}
      </dl>

      <div>
        <div className="flex h-12 items-end gap-[3px]">
          {history.map((p, i) => (
            <span
              key={p.date}
              data-bar
              title={`${dayFormat.format(p.date)} · ${p.value}`}
              className={cn(
                "flex-1 rounded-[3px] transition-opacity hover:opacity-100",
                i === history.length - 1 ? "opacity-100" : "opacity-55",
              )}
              style={{
                height: `${Math.max(8, p.value)}%`,
                background: MOOD_COLORS[moodLevel(p.value)],
              }}
            />
          ))}
        </div>
        <div className="mt-1.5 flex justify-between text-[11px] text-muted-foreground">
          <span>{t("market.days", { count: history.length })}</span>
          <a
            href="https://alternative.me/crypto/fear-and-greed-index/"
            target="_blank"
            rel="noreferrer"
            className="hover:text-foreground"
          >
            alternative.me
          </a>
        </div>
      </div>
    </div>
  );
}

function MoodSkeleton() {
  return (
    <div className="space-y-5">
      <Skeleton className="mx-auto h-[132px] w-[240px] rounded-t-full" />
      <div className="grid grid-cols-3 gap-2">
        <Skeleton className="h-14 rounded-xl" />
        <Skeleton className="h-14 rounded-xl" />
        <Skeleton className="h-14 rounded-xl" />
      </div>
      <Skeleton className="h-12 rounded-lg" />
    </div>
  );
}
