import { useId, useMemo } from "react";

/** Dependency-free SVG sparkline with a soft gradient fill. */
export function Sparkline({
  data,
  positive,
  width = 96,
  height = 32,
}: {
  data: number[];
  positive: boolean;
  width?: number;
  height?: number;
}) {
  const id = useId();
  const color = positive ? "var(--success)" : "var(--danger)";

  const { line, area } = useMemo(() => {
    const min = Math.min(...data);
    const max = Math.max(...data);
    const span = max - min || 1;
    const pts = data.map((v, i) => [
      (i / (data.length - 1)) * width,
      height - 2 - ((v - min) / span) * (height - 4),
    ]);
    const d = pts.map(([x, y], i) => `${i ? "L" : "M"}${x?.toFixed(1)},${y?.toFixed(1)}`).join("");
    return { line: d, area: `${d}L${width},${height}L0,${height}Z` };
  }, [data, width, height]);

  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden="true">
      <defs>
        <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.28" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={area} fill={`url(#${id})`} />
      <path
        d={line}
        fill="none"
        stroke={color}
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
