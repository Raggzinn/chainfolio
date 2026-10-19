import { cn } from "@/lib/utils";
import { NETWORK_ICONS, type NetworkKey, TOKEN_ICONS } from "./crypto-icons";

/**
 * Official token logo with an optional chain badge. Falls back to a tinted
 * monogram for tokens we don't ship a logo for.
 */
export function TokenIcon({
  symbol,
  color = "#64748B",
  chain,
  size = 36,
  className,
}: {
  symbol: string;
  color?: string;
  chain?: NetworkKey;
  size?: number;
  className?: string;
}) {
  const Logo = TOKEN_ICONS[symbol.toUpperCase() as keyof typeof TOKEN_ICONS];
  const Network = chain ? NETWORK_ICONS[chain] : undefined;
  const badge = Math.round(size * 0.42);

  return (
    <span
      className={cn("relative inline-flex shrink-0", className)}
      style={{ width: size, height: size }}
    >
      {Logo ? (
        <Logo variant="background" size={size} className="rounded-full" />
      ) : (
        <span
          className="flex size-full items-center justify-center rounded-full font-semibold text-primary-foreground"
          style={{
            fontSize: size * 0.3,
            background: color,
          }}
        >
          {symbol.slice(0, 3)}
        </span>
      )}
      {Network && (
        <span
          className="absolute -right-1 -bottom-1 rounded-full bg-background p-[2px]"
          style={{ width: badge + 4, height: badge + 4 }}
        >
          <Network variant="background" size={badge} className="rounded-full" />
        </span>
      )}
    </span>
  );
}
