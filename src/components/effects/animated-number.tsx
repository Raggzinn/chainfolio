import { useRef } from "react";
import { MASK } from "@/lib/format";
import { gsap, prefersReducedMotion, useGSAP } from "@/lib/gsap";

/**
 * Tweens from the previously rendered value to `value`, formatting every
 * frame. Falls back to a static render when motion is reduced or hidden.
 */
export function AnimatedNumber({
  value,
  format,
  hidden = false,
  duration = 1.2,
  className,
}: {
  value: number;
  format: (n: number) => string;
  hidden?: boolean;
  duration?: number;
  className?: string;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const current = useRef({ n: 0 });
  // Read the latest formatter without restarting the tween when its identity changes.
  const formatRef = useRef(format);
  formatRef.current = format;
  // A count-up still running when the value gets hidden must never write over the mask.
  const tween = useRef<gsap.core.Tween | null>(null);
  const hiddenRef = useRef(hidden);
  hiddenRef.current = hidden;

  useGSAP(
    () => {
      const el = ref.current;
      if (!el) return;
      tween.current?.kill();
      tween.current = null;
      if (hidden) {
        el.textContent = MASK;
        return;
      }
      if (prefersReducedMotion()) {
        current.current.n = value;
        el.textContent = formatRef.current(value);
        return;
      }
      tween.current = gsap.to(current.current, {
        n: value,
        duration,
        ease: "expo.out",
        onUpdate: () => {
          if (!hiddenRef.current) el.textContent = formatRef.current(current.current.n);
        },
      });
    },
    { dependencies: [value, hidden], revertOnUpdate: false },
  );

  return (
    <span ref={ref} className={className}>
      {hidden ? MASK : format(current.current.n)}
    </span>
  );
}
