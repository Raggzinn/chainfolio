import { Link } from "@tanstack/react-router";
import { Code2, HardDrive, Plus, ShieldCheck, Sparkles } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand/brand-mark";
import { TokenIcon } from "@/components/token-icon";
import { gsap, MOTION_OK, useGSAP } from "@/lib/gsap";
import { usePreferences } from "@/stores/preferences";

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

export function Onboarding() {
  const { t } = useTranslation();
  const setDemoMode = usePreferences((s) => s.setDemoMode);
  const scope = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        const tl = gsap.timeline({ defaults: { ease: "expo.out" } });
        tl.from("[data-core]", { scale: 0.4, autoAlpha: 0, duration: 1.2 })
          .from("[data-ring]", { scale: 0.6, autoAlpha: 0, duration: 1.4, stagger: 0.12 }, "<0.1")
          .from("[data-copy] > *", { y: 24, autoAlpha: 0, duration: 0.9, stagger: 0.08 }, "<0.2");

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
      });
      return () => mm.revert();
    },
    { scope },
  );

  return (
    <div
      ref={scope}
      className="relative grid min-h-[calc(100vh-110px)] items-center gap-10 py-6 lg:grid-cols-[1.05fr_1fr]"
    >
      <div data-copy className="relative z-10 max-w-xl space-y-6">
        <span className="surface inline-flex items-center gap-2 rounded-full px-3 py-1.5 font-medium text-xs">
          <Sparkles className="size-3.5 text-success" />
          {t("onboarding.eyebrow")}
        </span>
        <h1 className="font-semibold text-5xl leading-[1.05] tracking-[-0.04em] xl:text-6xl">
          {t("onboarding.title")}
          <br />
          <span className="text-primary">{t("onboarding.titleAccent")}</span>
        </h1>
        <p className="max-w-lg text-lg text-muted-foreground leading-relaxed">
          {t("onboarding.subtitle")}
        </p>
        <div className="flex flex-wrap items-center gap-3 pt-2">
          <Link
            to="/wallets"
            className="group relative inline-flex h-12 items-center gap-2 overflow-hidden rounded-xl bg-primary px-5 font-semibold text-primary-foreground transition-all duration-300 hover:bg-primary/90 active:scale-[0.98]"
          >
            <Plus className="size-5" />
            {t("onboarding.addWallet")}
          </Link>
          <button
            type="button"
            onClick={() => setDemoMode(true)}
            className="surface inline-flex h-12 items-center gap-2 rounded-xl px-5 font-medium transition-all duration-300 hover:bg-accent"
          >
            {t("onboarding.exploreDemo")}
          </button>
        </div>
        <ul className="flex flex-wrap gap-x-6 gap-y-2 pt-4 text-muted-foreground text-sm">
          <Feature icon={ShieldCheck}>{t("onboarding.chipWatchOnly")}</Feature>
          <Feature icon={HardDrive}>{t("onboarding.chipLocal")}</Feature>
          <Feature icon={Code2}>{t("onboarding.chipOpenSource")}</Feature>
        </ul>
      </div>

      <div className="relative mx-auto flex aspect-square w-full max-w-[480px] items-center justify-center">
        {RINGS.map((ring) => (
          <div
            key={ring.radius}
            data-ring
            className="absolute rounded-full border border-foreground/[0.08] border-dashed"
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
        ))}
        <div data-core className="relative flex size-28 items-center justify-center">
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

function Feature({
  icon: Icon,
  children,
}: {
  icon: typeof ShieldCheck;
  children: React.ReactNode;
}) {
  return (
    <li className="flex items-center gap-2">
      <Icon className="size-4 text-success" />
      {children}
    </li>
  );
}
