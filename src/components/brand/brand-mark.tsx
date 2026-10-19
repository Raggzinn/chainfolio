import { cn } from "@/lib/utils";

const ARC_TOP = "M84.5 39.4 A32 32 0 0 0 28 60";
const ARC_BOTTOM = "M28 60 A32 32 0 0 0 84.5 80.6";

const VARIANTS = {
  /** Brand-blue tile; reads well on both themes, used inside the app. */
  inverted: { tile: "#1E6BFF", top: "#F5F7FA", bottom: "#9CC9FF", node: "#19E3A1" },
  /** Graphite tile, same as the desktop app icon. */
  dark: { tile: "#1C1D20", top: "#1E6BFF", bottom: "#9CC9FF", node: "#19E3A1" },
  light: { tile: "#F2F2F4", top: "#1E6BFF", bottom: "#9CC9FF", node: "#0E9F73" },
} as const;

/** The Chainfolio "arc" mark: an open C split in two tones with an on-chain node. */
export function BrandMark({
  variant = "inverted",
  className,
}: {
  variant?: keyof typeof VARIANTS;
  className?: string;
}) {
  const c = VARIANTS[variant];
  return (
    <svg viewBox="0 0 120 120" className={cn("size-8", className)} aria-hidden="true">
      <rect width="120" height="120" rx="28" fill={c.tile} />
      <path d={ARC_TOP} fill="none" stroke={c.top} strokeWidth="12" strokeLinecap="round" />
      <path d={ARC_BOTTOM} fill="none" stroke={c.bottom} strokeWidth="12" strokeLinecap="round" />
      <circle cx="93" cy="60" r="7" fill={c.node} />
    </svg>
  );
}
