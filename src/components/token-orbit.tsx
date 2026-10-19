import { useRef } from "react";
import { BrandMark } from "@/components/brand/brand-mark";
import { TokenIcon } from "@/components/token-icon";
import { gsap, MOTION_OK, useGSAP } from "@/lib/gsap";
import { cn } from "@/lib/utils";

const RINGS = [
  {
    radius: 92,
    duration: 28,
    tokens: [
      { symbol: "ETH", color: "#627EEA" },
      { symbol: "BTC", color: "#F7931A" },
      { symbol: "SOL", color: "#9945FF" },
    ],
  },
  {
    radius: 150,
    duration: 44,
    tokens: [
      { symbol: "ARB", color: "#28A0F0" },
      { symbol: "BASE", color: "#0052FF" },
      { symbol: "OP", color: "#FF0420" },
      { symbol: "POL", color: "#8247E5" },
    ],
  },
  {
    radius: 208,
    duration: 64,
    tokens: [
      { symbol: "AVAX", color: "#E84142" },
      { symbol: "BNB", color: "#F3BA2F" },
      { symbol: "USDC", color: "#2775CA" },
      { symbol: "LINK", color: "#2A5ADA" },
      { symbol: "UNI", color: "#FF007A" },
    ],
  },
];

/** Diameter of the outer ring plus room for its tokens, at scale 1. */
export const ORBIT_SIZE = 480;

/**
 * Brand mark orbited by token logos. With `parallax`, each ring drifts
 * against the pointer at a different depth.
 */
export function TokenOrbit({
  scale = 1,
  parallax = false,
  className,
}: {
  scale?: number;
  parallax?: boolean;
  className?: string;
}) {
  const scope = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        const tl = gsap.timeline({ defaults: { ease: "expo.out" } });
        tl.from("[data-core]", { scale: 0.4, autoAlpha: 0, duration: 1.2 }).from(
          "[data-ring]",
          { scale: 0.6, autoAlpha: 0, duration: 1.4, stagger: 0.12 },
          "<0.1",
        );

        // Rings spin forever; tokens counter-rotate to stay upright.
        gsap.utils.toArray<HTMLElement>("[data-ring]").forEach((ring, i) => {
          const duration = RINGS[i]?.duration ?? 40;
          const dir = i % 2 ? -1 : 1;
          gsap.to(ring, { rotate: 360 * dir, duration, ease: "none", repeat: -1 });
          gsap.to(ring.querySelectorAll("[data-token]"), {
            rotate: -360 * dir,
            duration,
            ease: "none",
            repeat: -1,
          });
        });

        gsap.to("[data-pulse]", {
          scale: 1.6,
          autoAlpha: 0,
          duration: 2.4,
          ease: "sine.out",
          repeat: -1,
          stagger: 0.8,
        });

        if (!parallax) return;
        const layers = gsap.utils.toArray<HTMLElement>("[data-depth]").map((el) => ({
          depth: Number(el.dataset.depth),
          x: gsap.quickTo(el, "x", { duration: 0.9, ease: "power3.out" }),
          y: gsap.quickTo(el, "y", { duration: 0.9, ease: "power3.out" }),
        }));
        const onMove = (e: PointerEvent) => {
          const dx = e.clientX / window.innerWidth - 0.5;
          const dy = e.clientY / window.innerHeight - 0.5;
          for (const layer of layers) {
            layer.x(dx * layer.depth * -60);
            layer.y(dy * layer.depth * -60);
          }
        };
        window.addEventListener("pointermove", onMove);
        return () => window.removeEventListener("pointermove", onMove);
      });
      return () => mm.revert();
    },
    { scope },
  );

  return (
    <div
      ref={scope}
      className={cn("relative flex items-center justify-center", className)}
      style={{ width: ORBIT_SIZE * scale, height: ORBIT_SIZE * scale }}
    >
      <div
        className="relative flex items-center justify-center"
        style={{ transform: `scale(${scale})`, width: ORBIT_SIZE, height: ORBIT_SIZE }}
      >
        {RINGS.map((ring, depth) => (
          <div
            key={ring.radius}
            data-depth={depth + 1}
            className="absolute flex items-center justify-center"
          >
            <div
              data-ring
              className="rounded-full border border-foreground/[0.08] border-dashed"
              style={{ width: ring.radius * 2, height: ring.radius * 2 }}
            >
              {ring.tokens.map((token, i) => {
                const angle = (360 / ring.tokens.length) * i;
                return (
                  <div
                    key={token.symbol}
                    className="absolute top-1/2 left-1/2"
                    style={{
                      transform: `rotate(${angle}deg) translateX(${ring.radius}px) rotate(${-angle}deg)`,
                    }}
                  >
                    <div data-token className="-translate-x-1/2 -translate-y-1/2">
                      <TokenIcon symbol={token.symbol} color={token.color} size={44} />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
        <div
          data-core
          data-depth={0.4}
          className="relative flex size-28 items-center justify-center"
        >
          <span
            data-pulse
            className="absolute inset-0 rounded-[26px] border-2 border-brand-blue/50"
          />
          <span
            data-pulse
            className="absolute inset-0 rounded-[26px] border-2 border-brand-mint/50"
          />
          <BrandMark className="relative size-28" />
        </div>
      </div>
    </div>
  );
}
