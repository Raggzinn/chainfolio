import { ArrowLeft, ArrowRight, X } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { gsap, prefersReducedMotion } from "@/lib/gsap";
import { useTour } from "@/stores/tour";
import { placeTooltip, type Rect } from "./placement";
import { TOUR_STEPS, tourTarget } from "./tour-steps";

const PAD = 8;
const RADIUS = 16;
const TOOLTIP_WIDTH = 340;

/** Spotlight walkthrough over the live UI, driven by `useTour`. */
export function ProductTour() {
  const active = useTour((s) => s.active);
  return active ? <TourOverlay /> : null;
}

function TourOverlay() {
  const { t } = useTranslation();
  const { step, next, prev, stop } = useTour();
  const id = TOUR_STEPS[step] ?? "navigation";
  const [rect, setRect] = useState<Rect | null>(null);
  const hole = useRef<SVGRectElement>(null);
  const ring = useRef<HTMLDivElement>(null);
  const tip = useRef<HTMLDivElement>(null);
  const first = useRef(true);

  const measure = useCallback(() => {
    const el = document.querySelector<HTMLElement>(tourTarget(id));
    if (!el) return setRect(null);
    const r = el.getBoundingClientRect();
    setRect({
      x: r.left - PAD,
      y: r.top - PAD,
      width: r.width + PAD * 2,
      height: r.height + PAD * 2,
    });
  }, [id]);

  // Bring the target into view, then keep the spotlight glued to it while things move.
  useEffect(() => {
    const el = document.querySelector<HTMLElement>(tourTarget(id));
    if (!el) {
      // The page may still be rendering (e.g. demo mode just turned on).
      const retry = window.setTimeout(measure, 300);
      return () => window.clearTimeout(retry);
    }
    el.scrollIntoView({ block: "center", behavior: prefersReducedMotion() ? "auto" : "smooth" });
    measure();
    const onChange = () => measure();
    window.addEventListener("resize", onChange);
    window.addEventListener("scroll", onChange, true);
    const ro = new ResizeObserver(onChange);
    ro.observe(el);
    return () => {
      window.removeEventListener("resize", onChange);
      window.removeEventListener("scroll", onChange, true);
      ro.disconnect();
    };
  }, [id, measure]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") stop();
      else if (e.key === "ArrowRight" || e.key === "Enter") next();
      else if (e.key === "ArrowLeft") prev();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [next, prev, stop]);

  useLayoutEffect(() => {
    if (!rect || !tip.current) return;
    const duration = first.current || prefersReducedMotion() ? 0 : 0.5;
    const box = { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
    gsap.to(hole.current, { attr: box, duration, ease: "expo.out", overwrite: true });
    gsap.to(ring.current, {
      left: rect.x,
      top: rect.y,
      width: rect.width,
      height: rect.height,
      autoAlpha: 1,
      duration,
      ease: "expo.out",
      overwrite: true,
    });
    const pos = placeTooltip(
      rect,
      { width: TOOLTIP_WIDTH, height: tip.current.offsetHeight },
      { width: window.innerWidth, height: window.innerHeight },
    );
    gsap.to(tip.current, {
      left: pos.x,
      top: pos.y,
      autoAlpha: 1,
      duration,
      ease: "expo.out",
      overwrite: true,
    });
    first.current = false;
  }, [rect]);

  // Pop the tooltip content on every step change.
  // biome-ignore lint/correctness/useExhaustiveDependencies: re-runs on purpose when the step changes
  useLayoutEffect(() => {
    if (prefersReducedMotion() || !tip.current) return;
    gsap.fromTo(
      tip.current.querySelectorAll("[data-tip-content] > *"),
      { y: 8, autoAlpha: 0 },
      { y: 0, autoAlpha: 1, duration: 0.4, stagger: 0.04, ease: "power3.out", delay: 0.1 },
    );
  }, [step]);

  const lastStep = step === TOUR_STEPS.length - 1;

  return (
    <div
      className="fixed inset-0 z-50"
      role="dialog"
      aria-modal="true"
      aria-labelledby="tour-title"
    >
      <svg className="absolute inset-0 size-full" aria-hidden="true">
        <defs>
          <mask id="tour-mask">
            <rect width="100%" height="100%" fill="#ffffff" />
            <rect ref={hole} x={0} y={0} width={0} height={0} rx={RADIUS} fill="#000000" />
          </mask>
        </defs>
        <rect width="100%" height="100%" fill="rgb(11 11 13 / 0.62)" mask="url(#tour-mask)" />
      </svg>
      {/* Clicks outside the target do nothing, so the tour can't get lost behind a navigation. */}
      <div className="absolute inset-0" />
      <div
        ref={ring}
        className="pointer-events-none invisible absolute rounded-2xl ring-2 ring-primary ring-offset-0"
      />
      <div
        ref={tip}
        style={{ width: TOOLTIP_WIDTH }}
        className="surface-raised invisible absolute rounded-2xl p-5"
      >
        <div data-tip-content className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="tabular font-medium text-primary text-xs">
              {t("tour.progress", { current: step + 1, total: TOUR_STEPS.length })}
            </span>
            <button
              type="button"
              onClick={stop}
              aria-label={t("tour.close")}
              className="-mr-1.5 rounded-md p-1 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              <X className="size-4" />
            </button>
          </div>
          <h3 id="tour-title" className="font-semibold text-base tracking-tight">
            {t(`tour.steps.${id}.title`)}
          </h3>
          <p className="text-muted-foreground text-sm leading-relaxed">
            {t(`tour.steps.${id}.body`)}
          </p>
        </div>
        <div className="mt-4 flex items-center justify-between">
          <div className="flex gap-1">
            {TOUR_STEPS.map((s, i) => (
              <span
                key={s}
                className={`h-1.5 rounded-full transition-all duration-300 ${i === step ? "w-4 bg-primary" : "w-1.5 bg-border"}`}
              />
            ))}
          </div>
          <div className="flex gap-1.5">
            {step > 0 && (
              <button
                type="button"
                onClick={prev}
                aria-label={t("welcome.back")}
                className="rounded-lg p-2 transition-colors hover:bg-accent"
              >
                <ArrowLeft className="size-4" />
              </button>
            )}
            <button
              type="button"
              onClick={next}
              className="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 font-semibold text-primary-foreground text-sm transition-colors hover:bg-primary/90"
            >
              {lastStep ? t("tour.finish") : t("welcome.next")}
              {!lastStep && <ArrowRight className="size-4" />}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
