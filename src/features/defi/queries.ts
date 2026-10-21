import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

export const POSITIONS_KEY = ["positions"] as const;

/** DeFi and staking positions from SQLite; refreshed together with balances. */
export function usePositions() {
  return useQuery({
    queryKey: POSITIONS_KEY,
    queryFn: api.getPositions,
    enabled: IS_TAURI,
    staleTime: Number.POSITIVE_INFINITY,
  });
}
