import { useQuery } from "@tanstack/react-query";

/**
 * Crypto Fear & Greed Index by alternative.me — free, keyless, updated daily.
 * https://alternative.me/crypto/fear-and-greed-index/
 */
const ENDPOINT = "https://api.alternative.me/fng/?limit=31";

export type MoodLevel = "extremeFear" | "fear" | "neutral" | "greed" | "extremeGreed";

export interface MoodPoint {
  value: number;
  level: MoodLevel;
  /** Unix ms at the start of the day the reading refers to. */
  date: number;
}

export interface MarketMood {
  today: MoodPoint;
  yesterday?: MoodPoint;
  lastWeek?: MoodPoint;
  lastMonth?: MoodPoint;
  /** Oldest → newest, up to 31 days. */
  history: MoodPoint[];
  /** Milliseconds until the index publishes its next reading. */
  nextUpdateIn?: number;
}

interface ApiResponse {
  data?: { value: string; timestamp: string; time_until_update?: string }[];
}

/** Same bands alternative.me uses for its labels. */
export function moodLevel(value: number): MoodLevel {
  if (value <= 24) return "extremeFear";
  if (value <= 46) return "fear";
  if (value <= 54) return "neutral";
  if (value <= 74) return "greed";
  return "extremeGreed";
}

export const MOOD_COLORS: Record<MoodLevel, string> = {
  extremeFear: "var(--danger)",
  fear: "var(--warning)",
  neutral: "var(--muted-foreground)",
  greed: "var(--chart-3)",
  extremeGreed: "var(--success)",
};

export function parseMarketMood(json: ApiResponse): MarketMood {
  const rows = json.data ?? [];
  const points: MoodPoint[] = rows
    .map((row) => {
      const value = Math.max(0, Math.min(100, Number(row.value)));
      return { value, level: moodLevel(value), date: Number(row.timestamp) * 1000 };
    })
    .filter((p) => Number.isFinite(p.value) && Number.isFinite(p.date));

  const [today, yesterday] = points;
  if (!today) throw new Error("Fear & Greed response has no data");

  const nextUpdate = Number(rows[0]?.time_until_update);
  return {
    today,
    yesterday,
    lastWeek: points[7],
    lastMonth: points[30],
    history: [...points].reverse(),
    nextUpdateIn: Number.isFinite(nextUpdate) ? nextUpdate * 1000 : undefined,
  };
}

export async function fetchMarketMood(signal?: AbortSignal): Promise<MarketMood> {
  const res = await fetch(ENDPOINT, { signal });
  if (!res.ok) throw new Error(`Fear & Greed request failed: ${res.status}`);
  return parseMarketMood(await res.json());
}

export function useMarketMood() {
  return useQuery({
    queryKey: ["market", "fear-greed"],
    queryFn: ({ signal }) => fetchMarketMood(signal),
    staleTime: 30 * 60_000,
    refetchInterval: 60 * 60_000,
    retry: 2,
  });
}
