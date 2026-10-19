import { useNavigate } from "@tanstack/react-router";
import {
  ArrowLeft,
  ArrowRight,
  Code2,
  Compass,
  FlaskConical,
  HardDrive,
  type LucideIcon,
  Plus,
  Receipt,
  ShieldCheck,
} from "lucide-react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand/brand-mark";
import { SegmentedControl } from "@/components/segmented-control";
import { Switch } from "@/components/ui/switch";
import { useTheme } from "@/hooks/use-theme";
import { type Language, SUPPORTED_LANGUAGES } from "@/i18n";
import { gsap, MOTION_OK, useGSAP } from "@/lib/gsap";
import { cn } from "@/lib/utils";
import { CURRENCIES, THEMES, usePreferences } from "@/stores/preferences";
import { useTour } from "@/stores/tour";

const STEPS = ["welcome", "privacy", "personalize", "modules", "ready"] as const;
type Step = (typeof STEPS)[number];

/** First-run walkthrough. Shown until it is finished or skipped. */
export function WelcomeScreen() {
  const welcomeCompleted = usePreferences((s) => s.welcomeCompleted);
  if (welcomeCompleted) return null;
  return <WelcomeFlow />;
}

function WelcomeFlow() {
  const { t } = useTranslation();
  const setWelcomeCompleted = usePreferences((s) => s.setWelcomeCompleted);
  const [index, setIndex] = useState(0);
  const direction = useRef(1);
  const root = useRef<HTMLDivElement>(null);
  const step: Step = STEPS[index] ?? "welcome";
  const last = index === STEPS.length - 1;

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap.from("[data-welcome-card]", {
          y: 32,
          scale: 0.97,
          autoAlpha: 0,
          duration: 0.8,
          ease: "expo.out",
        });
      });
      return () => mm.revert();
    },
    { scope: root },
  );

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap.from("[data-step] > *", {
          x: 28 * direction.current,
          autoAlpha: 0,
          duration: 0.55,
          stagger: 0.05,
          ease: "expo.out",
        });
      });
      return () => mm.revert();
    },
    { scope: root, dependencies: [index] },
  );

  const go = (next: number) => {
    direction.current = next > index ? 1 : -1;
    setIndex(Math.max(0, Math.min(STEPS.length - 1, next)));
  };

  const finish = () => setWelcomeCompleted(true);

  return (
    <div
      ref={root}
      role="dialog"
      aria-modal="true"
      aria-labelledby="welcome-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/96 p-6"
    >
      <div
        data-welcome-card
        className="surface-raised relative w-full max-w-[640px] overflow-hidden rounded-3xl"
      >
        <div className="flex items-center justify-between px-7 pt-6">
          <Progress index={index} />
          {!last && (
            <button
              type="button"
              onClick={finish}
              className="rounded-lg px-2 py-1 font-medium text-muted-foreground text-xs transition-colors hover:bg-accent hover:text-foreground"
            >
              {t("welcome.skip")}
            </button>
          )}
        </div>

        <div key={step} data-step className="h-[420px] space-y-6 px-7 pt-6 pb-2">
          {step === "welcome" && <WelcomeStep />}
          {step === "privacy" && <PrivacyStep />}
          {step === "personalize" && <PersonalizeStep />}
          {step === "modules" && <ModulesStep />}
          {step === "ready" && <ReadyStep onDone={finish} />}
        </div>

        <div className="flex items-center justify-between border-t px-7 py-4">
          <button
            type="button"
            onClick={() => go(index - 1)}
            disabled={index === 0}
            className="inline-flex items-center gap-1.5 rounded-lg px-3 py-2 font-medium text-sm transition-colors hover:bg-accent disabled:invisible"
          >
            <ArrowLeft className="size-4" />
            {t("welcome.back")}
          </button>
          {!last && (
            <button
              type="button"
              onClick={() => go(index + 1)}
              className="inline-flex items-center gap-1.5 rounded-xl bg-primary px-4 py-2 font-semibold text-primary-foreground text-sm transition-colors hover:bg-primary/90"
            >
              {index === 0 ? t("welcome.start") : t("welcome.next")}
              <ArrowRight className="size-4" />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function Progress({ index }: { index: number }) {
  const bar = useRef<HTMLSpanElement>(null);
  useGSAP(
    () => {
      gsap.to(bar.current, {
        width: `${((index + 1) / STEPS.length) * 100}%`,
        duration: 0.6,
        ease: "expo.out",
      });
    },
    { dependencies: [index] },
  );
  return (
    <div className="flex items-center gap-3">
      <div className="h-1.5 w-32 overflow-hidden rounded-full bg-muted">
        <span ref={bar} className="block h-full w-0 rounded-full bg-primary" />
      </div>
      <span className="tabular text-muted-foreground text-xs">
        {index + 1}/{STEPS.length}
      </span>
    </div>
  );
}

function Heading({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <div className="space-y-2">
      <h2 id="welcome-title" className="font-semibold text-[28px] leading-tight tracking-[-0.03em]">
        {title}
      </h2>
      <p className="text-muted-foreground leading-relaxed">{subtitle}</p>
    </div>
  );
}

function WelcomeStep() {
  const { t } = useTranslation();
  const mark = useRef<HTMLDivElement>(null);
  useGSAP(() => {
    const mm = gsap.matchMedia();
    mm.add(MOTION_OK, () => {
      gsap.from(mark.current, {
        scale: 0.5,
        rotate: -25,
        duration: 1.2,
        ease: "elastic.out(1, 0.55)",
      });
    });
    return () => mm.revert();
  });
  return (
    <>
      <div ref={mark} className="pt-2">
        <BrandMark className="size-20" />
      </div>
      <Heading title={t("welcome.welcome.title")} subtitle={t("welcome.welcome.subtitle")} />
      <ul className="grid grid-cols-3 gap-3">
        {(["chains", "pnl", "risk"] as const).map((key) => (
          <li key={key} className="rounded-2xl bg-muted px-4 py-3">
            <p className="font-semibold text-sm">{t(`welcome.welcome.${key}.title`)}</p>
            <p className="mt-0.5 text-muted-foreground text-xs leading-relaxed">
              {t(`welcome.welcome.${key}.body`)}
            </p>
          </li>
        ))}
      </ul>
    </>
  );
}

function PrivacyStep() {
  const { t } = useTranslation();
  const items: { key: "watchOnly" | "local" | "openSource"; icon: LucideIcon }[] = [
    { key: "watchOnly", icon: ShieldCheck },
    { key: "local", icon: HardDrive },
    { key: "openSource", icon: Code2 },
  ];
  return (
    <>
      <Heading title={t("welcome.privacy.title")} subtitle={t("welcome.privacy.subtitle")} />
      <ul className="space-y-3">
        {items.map(({ key, icon: Icon }) => (
          <li key={key} className="flex items-start gap-4 rounded-2xl bg-muted p-4">
            <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-success/12 text-success">
              <Icon className="size-5" />
            </span>
            <div>
              <p className="font-semibold text-sm">{t(`welcome.privacy.${key}.title`)}</p>
              <p className="mt-0.5 text-muted-foreground text-sm leading-relaxed">
                {t(`welcome.privacy.${key}.body`)}
              </p>
            </div>
          </li>
        ))}
      </ul>
    </>
  );
}

function PersonalizeStep() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const currency = usePreferences((s) => s.currency);
  const setCurrency = usePreferences((s) => s.setCurrency);
  return (
    <>
      <Heading
        title={t("welcome.personalize.title")}
        subtitle={t("welcome.personalize.subtitle")}
      />
      <div className="divide-y divide-border rounded-2xl bg-muted px-4">
        <Row label={t("settings.language")}>
          <SegmentedControl
            label={t("settings.language")}
            options={SUPPORTED_LANGUAGES.map((l) => l.code)}
            labels={{ "pt-BR": "Português", en: "English" }}
            value={(i18n.resolvedLanguage ?? "en") as Language}
            onChange={(v) => i18n.changeLanguage(v)}
            className="bg-card"
          />
        </Row>
        <Row label={t("settings.theme")}>
          <SegmentedControl
            label={t("settings.theme")}
            options={THEMES}
            labels={{
              dark: t("settings.themeDark"),
              light: t("settings.themeLight"),
              system: t("settings.themeSystem"),
            }}
            value={theme}
            onChange={setTheme}
            className="bg-card"
          />
        </Row>
        <Row label={t("settings.currency")}>
          <SegmentedControl
            label={t("settings.currency")}
            options={CURRENCIES}
            value={currency}
            onChange={setCurrency}
            className="bg-card"
          />
        </Row>
      </div>
    </>
  );
}

function ModulesStep() {
  const { t } = useTranslation();
  const taxModuleEnabled = usePreferences((s) => s.taxModuleEnabled);
  const setTaxModuleEnabled = usePreferences((s) => s.setTaxModuleEnabled);
  return (
    <>
      <Heading title={t("welcome.modules.title")} subtitle={t("welcome.modules.subtitle")} />
      <label
        htmlFor="welcome-tax"
        className={cn(
          "flex cursor-pointer items-start gap-4 rounded-2xl border-2 p-4 transition-colors",
          taxModuleEnabled ? "border-primary bg-primary/8" : "border-transparent bg-muted",
        )}
      >
        <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary/12 text-primary">
          <Receipt className="size-5" />
        </span>
        <div className="flex-1">
          <p className="font-semibold text-sm">{t("settings.taxModule")}</p>
          <p className="mt-0.5 text-muted-foreground text-sm leading-relaxed">
            {t("welcome.modules.taxBody")}
          </p>
        </div>
        <Switch id="welcome-tax" checked={taxModuleEnabled} onCheckedChange={setTaxModuleEnabled} />
      </label>
      <p className="text-muted-foreground text-xs">{t("welcome.modules.later")}</p>
    </>
  );
}

function ReadyStep({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const setDemoMode = usePreferences((s) => s.setDemoMode);
  const startTour = useTour((s) => s.start);

  const actions: {
    key: "wallet" | "demo" | "tour";
    icon: LucideIcon;
    primary?: boolean;
    run: () => void;
  }[] = [
    {
      key: "wallet",
      icon: Plus,
      primary: true,
      run: () => {
        onDone();
        navigate({ to: "/wallets" });
      },
    },
    {
      key: "tour",
      icon: Compass,
      run: () => {
        onDone();
        navigate({ to: "/" });
        startTour();
      },
    },
    {
      key: "demo",
      icon: FlaskConical,
      run: () => {
        setDemoMode(true);
        onDone();
        navigate({ to: "/" });
      },
    },
  ];

  return (
    <>
      <Heading title={t("welcome.ready.title")} subtitle={t("welcome.ready.subtitle")} />
      <div className="space-y-3">
        {actions.map(({ key, icon: Icon, primary, run }) => (
          <button
            key={key}
            type="button"
            onClick={run}
            className={cn(
              "group flex w-full items-center gap-4 rounded-2xl p-4 text-left transition-all duration-200 active:scale-[0.99]",
              primary
                ? "bg-primary text-primary-foreground hover:bg-primary/90"
                : "bg-muted hover:bg-accent",
            )}
          >
            <span
              className={cn(
                "flex size-10 shrink-0 items-center justify-center rounded-xl",
                primary ? "bg-primary-foreground/15" : "bg-card",
              )}
            >
              <Icon className="size-5" />
            </span>
            <span className="flex-1">
              <span className="block font-semibold text-sm">{t(`welcome.ready.${key}.title`)}</span>
              <span
                className={cn(
                  "block text-sm",
                  primary ? "text-primary-foreground/80" : "text-muted-foreground",
                )}
              >
                {t(`welcome.ready.${key}.body`)}
              </span>
            </span>
            <ArrowRight className="size-4 transition-transform group-hover:translate-x-0.5" />
          </button>
        ))}
      </div>
    </>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-3.5">
      <span className="font-medium text-sm">{label}</span>
      {children}
    </div>
  );
}
