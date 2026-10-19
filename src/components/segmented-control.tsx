import { useRef } from "react";
import { gsap, useGSAP } from "@/lib/gsap";
import { cn } from "@/lib/utils";

/** Pill tabs with a thumb that glides between options. */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  className,
  label,
  labels,
}: {
  options: readonly T[];
  labels?: Partial<Record<T, string>>;
  value: T;
  onChange: (value: T) => void;
  className?: string;
  label: string;
}) {
  const root = useRef<HTMLDivElement>(null);
  const thumb = useRef<HTMLSpanElement>(null);
  const mounted = useRef(false);

  useGSAP(
    () => {
      const target = root.current?.querySelector<HTMLElement>(`[data-value="${value}"]`);
      if (!target || !thumb.current) return;
      gsap.to(thumb.current, {
        x: target.offsetLeft,
        width: target.offsetWidth,
        duration: mounted.current ? 0.45 : 0,
        ease: "expo.out",
      });
      mounted.current = true;
    },
    { dependencies: [value, options.length] },
  );

  return (
    <div
      ref={root}
      role="tablist"
      aria-label={label}
      className={cn("relative inline-flex rounded-xl bg-muted p-1", className)}
    >
      <span
        ref={thumb}
        aria-hidden="true"
        className="absolute top-1 bottom-1 left-0 rounded-lg bg-card shadow-surface ring-1 ring-border"
      />
      {options.map((option) => (
        <button
          key={option}
          type="button"
          role="tab"
          aria-selected={option === value}
          data-value={option}
          onClick={() => onChange(option)}
          className={cn(
            "relative z-10 h-7 rounded-lg px-3 font-medium text-xs transition-colors duration-200",
            option === value ? "text-foreground" : "text-muted-foreground hover:text-foreground",
          )}
        >
          {labels?.[option] ?? option}
        </button>
      ))}
    </div>
  );
}
