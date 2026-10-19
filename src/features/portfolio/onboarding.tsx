import { Link } from "@tanstack/react-router";
import { Code2, HardDrive, Plus, ShieldCheck, Sparkles } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { TokenOrbit } from "@/components/token-orbit";
import { gsap, MOTION_OK, useGSAP } from "@/lib/gsap";
import { usePreferences } from "@/stores/preferences";

export function Onboarding() {
  const { t } = useTranslation();
  const setDemoMode = usePreferences((s) => s.setDemoMode);
  const scope = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        const tl = gsap.timeline({ defaults: { ease: "expo.out" } });
        tl.from("[data-copy] > *", { y: 24, autoAlpha: 0, duration: 0.9, stagger: 0.08 }, 0.2);
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

      <TokenOrbit parallax className="mx-auto" />
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
