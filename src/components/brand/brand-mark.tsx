import { useId } from "react";
import { cn } from "@/lib/utils";

/** Temporary app mark, replaced once the final logo concept is chosen. */
export function BrandMark({ className }: { className?: string }) {
  const id = useId();
  return (
    <svg viewBox="0 0 32 32" className={cn("size-8", className)} aria-hidden="true">
      <defs>
        <linearGradient id={id} x1="0" y1="32" x2="32" y2="0" gradientUnits="userSpaceOnUse">
          <stop stopColor="#1E6BFF" />
          <stop offset="1" stopColor="#19E3A1" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="8" fill={`url(#${id})`} />
      <rect x="8" y="17" width="4" height="7" rx="1.5" fill="#fff" fillOpacity=".7" />
      <rect x="14" y="12" width="4" height="12" rx="1.5" fill="#fff" fillOpacity=".85" />
      <rect x="20" y="8" width="4" height="16" rx="1.5" fill="#fff" />
    </svg>
  );
}
