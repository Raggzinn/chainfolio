import { Code2, HardDrive, type LucideIcon, ShieldCheck } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand/brand-mark";
import { ChangePill } from "@/components/change-pill";
import { AnimatedNumber } from "@/components/effects/animated-number";
import { Sparkline } from "@/components/sparkline";
import { TokenIcon } from "@/components/token-icon";
import { TokenOrbit } from "@/components/token-orbit";
import { assetValue, DEMO_ASSETS, DEMO_TOTAL, demoPortfolioHistory } from "@/features/demo/data";
import { useMoney } from "@/hooks/use-money";
import { gsap, MOTION_OK, prefersReducedMotion, useGSAP } from "@/lib/gsap";
import { cn } from "@/lib/utils";
import { usePreferences } from "@/stores/preferences";

/* Step 1 — the orbit, following the pointer. */
export function OrbitStage() {
  return <TokenOrbit parallax scale={1.1} />;
}

/* Step 2 — a shield with privacy facts floating around it. */
export function PrivacyStage() {
  const { t } = useTranslation();
  const scope = useRef<HTMLDivElement>(null);
  const cards: {
    key: "watchOnly" | "local" | "openSource";
    icon: LucideIcon;
    className: string;
  }[] = [
    { key: "watchOnly", icon: ShieldCheck, className: "top-[14%] left-[6%]" },
    { key: "local", icon: HardDrive, className: "top-[40%] right-[4%]" },
    { key: "openSource", icon: Code2, className: "bottom-[12%] left-[12%]" },
  ];

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap.from("[data-shield]", {
          scale: 0.3,
          rotate: -20,
          duration: 1.2,
          ease: "elastic.out(1, 0.6)",
        });
        gsap.from("[data-fact]", {
          y: 40,
          autoAlpha: 0,
          duration: 0.9,
          stagger: 0.15,
          ease: "expo.out",
          delay: 0.2,
        });
        gsap.utils.toArray<HTMLElement>("[data-fact]").forEach((el, i) => {
          gsap.to(el, {
            y: i % 2 ? 10 : -10,
            duration: 2.6 + i * 0.4,
            ease: "sine.inOut",
            repeat: -1,
            yoyo: true,
            delay: 1,
          });
        });
        gsap.to("[data-ring-pulse]", {
          scale: 1.9,
          autoAlpha: 0,
          duration: 2.8,
          ease: "sine.out",
          repeat: -1,
          stagger: 0.9,
        });
      });
      return () => mm.revert();
    },
    { scope },
  );

  return (
    <div ref={scope} className="relative size-full">
      <div className="absolute inset-0 flex items-center justify-center">
        <div className="relative">
          <span
            data-ring-pulse
            className="absolute inset-0 rounded-[40px] border-2 border-success/50"
          />
          <span
            data-ring-pulse
            className="absolute inset-0 rounded-[40px] border-2 border-success/50"
          />
          <div
            data-shield
            className="relative flex size-44 items-center justify-center rounded-[40px] bg-success/12 text-success"
          >
            <ShieldCheck className="size-20" strokeWidth={1.6} />
          </div>
        </div>
      </div>
      {cards.map(({ key, icon: Icon, className }) => (
        <div
          key={key}
          data-fact
          className={cn(
            "surface absolute flex max-w-60 items-center gap-3 rounded-2xl p-4 transition-transform duration-300 hover:scale-105",
            className,
          )}
        >
          <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-success/12 text-success">
            <Icon className="size-5" />
          </span>
          <span className="font-semibold text-sm">{t(`welcome.privacy.${key}.title`)}</span>
        </div>
      ))}
    </div>
  );
}

/* Step 3 — a live preview that follows the language, theme and currency picked on the left. */
export function PreviewStage() {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const series = demoPortfolioHistory("1M").map((p) => p.value);
  const assets = [...DEMO_ASSETS].sort((a, b) => assetValue(b) - assetValue(a)).slice(0, 3);
  const amount = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumFractionDigits: 3 });
  const scope = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap.from("[data-preview] > *", {
          y: 30,
          autoAlpha: 0,
          duration: 0.8,
          stagger: 0.08,
          ease: "expo.out",
        });
      });
      return () => mm.revert();
    },
    { scope },
  );

  return (
    <div ref={scope} className="flex size-full items-center justify-center p-10">
      <div data-preview className="w-full max-w-md space-y-3">
        <span className="inline-flex items-center gap-2 rounded-full bg-primary/12 px-3 py-1 font-medium text-primary text-xs">
          <span className="size-1.5 animate-pulse rounded-full bg-primary" />
          {t("welcome.personalize.preview")}
        </span>
        <div className="surface space-y-3 rounded-3xl p-6">
          <p className="text-muted-foreground text-sm">{t("dashboard.totalValue")}</p>
          <AnimatedNumber
            value={DEMO_TOTAL}
            format={(n) => format(n, { ignorePrivacy: true })}
            className="tabular block font-semibold text-4xl tracking-tight"
          />
          <div className="flex items-center gap-2 text-sm">
            <ChangePill value={0.0412} />
            <span className="text-muted-foreground">{t("dashboard.today")}</span>
          </div>
          <Sparkline data={series} positive width={380} height={90} />
        </div>
        <div className="surface divide-y divide-border rounded-3xl px-5">
          {assets.map((a) => (
            <div key={a.symbol} className="flex items-center gap-3 py-3">
              <TokenIcon symbol={a.symbol} color={a.color} chain={a.chain} size={32} />
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium text-sm">{a.name}</p>
                <p className="text-muted-foreground text-xs">
                  {amount.format(a.balance)} {a.symbol}
                </p>
              </div>
              <span className="tabular font-semibold text-sm">
                {format(assetValue(a), { ignorePrivacy: true })}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

const MONTHLY_SALES_BRL = [12_400, 41_800, 8_200, 33_900, 22_100, 52_600];
const EXEMPTION_BRL = 35_000;
const CHART_MAX = 60_000;

/* Step 4 — a tax report that lights up when the module is turned on. */
export function TaxStage() {
  const { t, i18n } = useTranslation();
  const enabled = usePreferences((s) => s.taxModuleEnabled);
  const scope = useRef<HTMLDivElement>(null);
  const month = new Intl.DateTimeFormat(i18n.resolvedLanguage, { month: "short" });
  const brl = new Intl.NumberFormat(i18n.resolvedLanguage, {
    style: "currency",
    currency: "BRL",
    notation: "compact",
  });

  useGSAP(
    () => {
      const instant = prefersReducedMotion();
      gsap.fromTo(
        "[data-tax-bar]",
        { scaleY: 0 },
        {
          scaleY: 1,
          transformOrigin: "bottom center",
          duration: instant ? 0 : 0.9,
          stagger: 0.07,
          ease: "expo.out",
        },
      );
      gsap.to("[data-tax-card]", {
        filter: enabled ? "grayscale(0)" : "grayscale(1)",
        opacity: enabled ? 1 : 0.45,
        scale: enabled ? 1 : 0.97,
        duration: instant ? 0 : 0.5,
        ease: "power2.out",
      });
    },
    { scope, dependencies: [enabled] },
  );

  return (
    <div ref={scope} className="flex size-full items-center justify-center p-10">
      <div data-tax-card className="surface w-full max-w-md space-y-5 rounded-3xl p-6">
        <div>
          <p className="font-semibold">{t("welcome.modules.previewTitle")}</p>
          <p className="text-muted-foreground text-sm">{t("welcome.modules.previewSubtitle")}</p>
        </div>
        <div className="space-y-2">
          <div className="relative h-44">
            <div
              className="absolute inset-x-0 z-10 border-warning border-t border-dashed"
              style={{ bottom: `${(EXEMPTION_BRL / CHART_MAX) * 100}%` }}
            >
              <span className="absolute -top-5 right-0 font-medium text-[11px] text-warning">
                R$ 35 mil
              </span>
            </div>
            <div className="flex h-full items-end gap-3">
              {MONTHLY_SALES_BRL.map((value) => (
                <div
                  key={value}
                  data-tax-bar
                  title={brl.format(value)}
                  className={cn(
                    "flex-1 rounded-lg",
                    value > EXEMPTION_BRL ? "bg-warning" : "bg-success",
                  )}
                  style={{ height: `${(value / CHART_MAX) * 100}%` }}
                />
              ))}
            </div>
          </div>
          <div className="flex gap-3">
            {MONTHLY_SALES_BRL.map((value, i) => (
              <span
                key={value}
                className="flex-1 text-center text-[11px] text-muted-foreground capitalize"
              >
                {month.format(new Date(2026, i + 3, 1)).replace(".", "")}
              </span>
            ))}
          </div>
        </div>
        <div className="flex gap-4 text-xs">
          <span className="flex items-center gap-1.5">
            <span className="size-2 rounded-full bg-success" />
            {t("welcome.modules.exempt")}
          </span>
          <span className="flex items-center gap-1.5">
            <span className="size-2 rounded-full bg-warning" />
            {t("welcome.modules.taxable")}
          </span>
        </div>
      </div>
    </div>
  );
}

const BURST = [
  "ETH",
  "BTC",
  "SOL",
  "USDC",
  "ARB",
  "OP",
  "POL",
  "AVAX",
  "BNB",
  "LINK",
  "UNI",
  "BASE",
];

/* Step 5 — the mark with token logos bursting out of it. Click to replay. */
export function ReadyStage() {
  const scope = useRef<HTMLDivElement>(null);

  const { contextSafe } = useGSAP(
    () => {
      burst();
    },
    { scope },
  );

  const burst = contextSafe(() => {
    if (prefersReducedMotion()) return;
    const tokens = gsap.utils.toArray<HTMLElement>("[data-burst]");
    gsap.fromTo(
      "[data-ready-mark]",
      { scale: 0.6, rotate: -15 },
      { scale: 1, rotate: 0, duration: 1.1, ease: "elastic.out(1, 0.5)" },
    );
    tokens.forEach((el, i) => {
      const angle = (i / tokens.length) * Math.PI * 2 + gsap.utils.random(-0.2, 0.2);
      const distance = gsap.utils.random(150, 250);
      gsap.fromTo(
        el,
        { x: 0, y: 0, scale: 0.2, autoAlpha: 0, rotate: 0 },
        {
          x: Math.cos(angle) * distance,
          y: Math.sin(angle) * distance,
          scale: 1,
          autoAlpha: 1,
          rotate: gsap.utils.random(-30, 30),
          duration: 1.3,
          ease: "expo.out",
          delay: 0.1 + i * 0.02,
          onComplete: () => {
            gsap.to(el, {
              y: `+=${gsap.utils.random(-12, 12)}`,
              duration: 2.4,
              repeat: -1,
              yoyo: true,
              ease: "sine.inOut",
            });
          },
        },
      );
    });
  });

  return (
    <div ref={scope} className="size-full">
      <button
        type="button"
        onClick={burst}
        aria-label="Replay"
        className="relative flex size-full cursor-pointer items-center justify-center"
      >
        {BURST.map((symbol) => (
          <span key={symbol} data-burst className="invisible absolute">
            <TokenIcon symbol={symbol} size={48} />
          </span>
        ))}
        <span data-ready-mark className="relative">
          <BrandMark className="size-36" />
        </span>
      </button>
    </div>
  );
}
