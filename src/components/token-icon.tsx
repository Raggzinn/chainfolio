import { useState } from "react";
import { cn } from "@/lib/utils";
import { NETWORK_ICONS, type NetworkKey, TOKEN_ICONS } from "./crypto-icons";

/**
 * Official token logo with an optional chain badge. Tokens we don't ship a
 * logo for use `src` when given (e.g. from the token indexer), then fall back
 * to a tinted monogram.
 */
export function TokenIcon({
  symbol,
  color = "#64748B",
  chain,
  src,
  size = 36,
  className,
}: {
  symbol: string;
  color?: string;
  chain?: NetworkKey;
  src?: string | null;
  size?: number;
  className?: string;
}) {
  const [broken, setBroken] = useState(false);
  const Logo = TOKEN_ICONS[symbol.toUpperCase() as keyof typeof TOKEN_ICONS];
  const badge = Math.round(size * 0.42);

  return (
    <span
      className={cn("relative inline-flex shrink-0", className)}
      style={{ width: size, height: size }}
    >
      {Logo ? (
        <Logo variant="background" size={size} className="rounded-full" />
      ) : src && !broken ? (
        <img
          src={src}
          alt=""
          width={size}
          height={size}
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setBroken(true)}
          className="size-full rounded-full bg-muted object-cover"
        />
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
      {chain && (
        <span
          className="absolute -right-1 -bottom-1 rounded-full bg-background p-[2px]"
          style={{ width: badge + 4, height: badge + 4 }}
        >
          <NetworkIcon chain={chain} size={badge} />
        </span>
      )}
    </span>
  );
}

/** Round network logo on its own. */
export function NetworkIcon({
  chain,
  size = 20,
  className,
}: {
  chain: NetworkKey;
  size?: number;
  className?: string;
}) {
  const Network = NETWORK_ICONS[chain];
  return <Network variant="background" size={size} className={cn("rounded-full", className)} />;
}
