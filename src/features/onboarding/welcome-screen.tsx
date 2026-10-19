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
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand/brand-mark";
import { WindowControls } from "@/components/layout/window-controls";
import { SegmentedControl } from "@/components/segmented-control";
import { Switch } from "@/components/ui/switch";
import { useTheme } from "@/hooks/use-theme";
import { type Language, SUPPORTED_LANGUAGES } from "@/i18n";
import { gsap, MOTION_OK, useGSAP } from "@/lib/gsap";
import { cn } from "@/lib/utils";
import { CURRENCIES, THEMES, usePreferences } from "@/stores/preferences";
import { useTour } from "@/stores/tour";
import { OrbitStage, PreviewStage, PrivacyStage, ReadyStage, TaxStage } from "./welcome-stages";

const STEPS = ["welcome", "privacy", "personalize", "modules", "ready"] as const;
type Step = (typeof STEPS)[number];

const STAGES: Record<Step, () => React.ReactNode> = {
  welcome: OrbitStage,
  privacy: PrivacyStage,
  personalize: PreviewStage,
  modules: TaxStage,
  ready: ReadyStage,
};

/** Full-screen first-run walkthrough. Shown until it is finished or skipped. */
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
  const Stage = STAGES[step];

  const go = useCallback(
    (next: number) => {
      const clamped = Math.max(0, Math.min(STEPS.length - 1, next));
      if (clamped === index) return;
      direction.current = clamped > index ? 1 : -1;
      setIndex(clamped);
    },
    [index],
  );

  const finish = useCallback(() => {
    const el = root.current;
    if (!el || !window.matchMedia(MOTION_OK).matches) return setWelcomeCompleted(true);
    gsap.to(el, {
      autoAlpha: 0,
      scale: 1.03,
      duration: 0.45,
      ease: "power2.in",
      onComplete: () => setWelcomeCompleted(true),
    });
  }, [setWelcomeCompleted]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLElement && e.target.closest("input, textarea")) return;
      if (e.key === "ArrowRight") go(index + 1);
      else if (e.key === "ArrowLeft") go(index - 1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go, index]);

  // Opening: the whole screen fades up, header and stage settle in.
  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap
          .timeline({ defaults: { ease: "expo.out" } })
          .from(root.current, { autoAlpha: 0, duration: 0.5, ease: "power2.out" })
          .from(
            "[data-welcome-header] > *",
            { y: -16, autoAlpha: 0, duration: 0.8, stagger: 0.08 },
            0.1,
          )
          .from("[data-welcome-stage]", { scale: 0.94, autoAlpha: 0, duration: 1.1 }, 0.15);
      });
      return () => mm.revert();
    },
    { scope: root },
  );

  // Every step: words of the title rise in, the rest slides from the travel direction.
  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap
          .timeline({ defaults: { ease: "expo.out" } })
          .from("[data-word]", { yPercent: 110, rotate: 4, duration: 0.9, stagger: 0.045 })
          .from(
            "[data-reveal]",
            { x: 36 * direction.current, autoAlpha: 0, duration: 0.8, stagger: 0.07 },
            0.15,
          )
          .from("[data-stage-content]", { autoAlpha: 0, scale: 0.96, duration: 0.9 }, 0);
      });
      return () => mm.revert();
    },
    { scope: root, dependencies: [index] },
  );

  return (
    <div
      ref={root}
      role="dialog"
      aria-modal="true"
      aria-labelledby="welcome-title"
      className="fixed inset-0 z-50 flex flex-col bg-background"
    >
      <header
        data-tauri-drag-region
        data-welcome-header
        className="flex h-16 shrink-0 items-center gap-6 pl-6"
      >
        <div className="flex items-center gap-2.5">
          <BrandMark className="size-8" />
          <span className="font-semibold text-[17px] tracking-tight">
            Chain<span className="text-primary">folio</span>
          </span>
        </div>
        <div className="mx-auto">
          <SegmentedControl
            label={t("welcome.steps")}
            options={STEPS}
            labels={Object.fromEntries(STEPS.map((s) => [s, t(`welcome.stepLabels.${s}`)]))}
            value={step}
            onChange={(s) => go(STEPS.indexOf(s))}
          />
        </div>
        <div className="flex h-full items-center gap-2 pr-4">
          {!last && (
            <button
              type="button"
              onClick={finish}
              className="rounded-lg px-3 py-1.5 font-medium text-muted-foreground text-sm transition-colors hover:bg-accent hover:text-foreground"
            >
              {t("welcome.skip")}
            </button>
          )}
          <WindowControls />
        </div>
      </header>

      <main className="grid min-h-0 flex-1 gap-6 p-6 pt-2 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <section className="flex min-h-0 flex-col justify-center px-4 lg:px-12 xl:px-20">
          <div key={step} className="max-w-xl space-y-8">
            <p data-reveal className="tabular font-medium text-primary text-sm">
              {String(index + 1).padStart(2, "0")} / {String(STEPS.length).padStart(2, "0")}
            </p>
            <h1
              id="welcome-title"
              className="font-semibold text-5xl leading-[1.04] tracking-[-0.04em] xl:text-6xl"
            >
              <SplitWords text={t(`welcome.${step}.title`)} />
            </h1>
            <p data-reveal className="max-w-lg text-lg text-muted-foreground leading-relaxed">
              {t(`welcome.${step}.subtitle`)}
            </p>
            <div data-reveal>
              {step === "welcome" && <HighlightsControls />}
              {step === "privacy" && <PrivacyFacts />}
              {step === "personalize" && <PersonalizeControls />}
              {step === "modules" && <ModulesControls />}
              {step === "ready" && <ReadyActions onDone={finish} />}
            </div>
          </div>

          <nav className="mt-12 flex max-w-xl items-center gap-3">
            {index > 0 && (
              <button
                type="button"
                onClick={() => go(index - 1)}
                aria-label={t("welcome.back")}
                className="flex size-12 items-center justify-center rounded-xl border bg-card transition-colors hover:bg-accent"
              >
                <ArrowLeft className="size-5" />
              </button>
            )}
            {!last && (
              <button
                type="button"
                onClick={() => go(index + 1)}
                className="group inline-flex h-12 items-center gap-2 rounded-xl bg-primary px-6 font-semibold text-primary-foreground transition-colors hover:bg-primary/90"
              >
                {index === 0 ? t("welcome.start") : t("welcome.next")}
                <ArrowRight className="size-5 transition-transform group-hover:translate-x-0.5" />
              </button>
            )}
            <span className="ml-3 hidden text-muted-foreground text-xs xl:inline">
              {t("welcome.keyboardHint")}
            </span>
          </nav>
        </section>

        <section
          data-welcome-stage
          className="relative hidden min-h-0 overflow-hidden rounded-[32px] border bg-card lg:block"
        >
          <div
            key={step}
            data-stage-content
            className="absolute inset-0 flex items-center justify-center"
          >
            <Stage />
          </div>
        </section>
      </main>
    </div>
  );
}

/** Wraps each word so it can rise from behind a mask. */
function SplitWords({ text }: { text: string }) {
  return text.split(" ").map((word, i) => (
    <span
      // biome-ignore lint/suspicious/noArrayIndexKey: words can repeat; order is the identity
      key={`${word}-${i}`}
      className="-mb-[0.12em] inline-block overflow-hidden pb-[0.12em] align-bottom"
    >
      <span data-word className="inline-block">
        {word}
        {" "}
      </span>
    </span>
  ));
}

function HighlightsControls() {
  const { t } = useTranslation();
  return (
    <ul className="grid grid-cols-3 gap-3">
      {(["chains", "pnl", "risk"] as const).map((key) => (
        <li
          key={key}
          className="rounded-2xl border bg-card p-4 transition-transform duration-300 hover:-translate-y-1"
        >
          <p className="font-semibold text-sm">{t(`welcome.welcome.${key}.title`)}</p>
          <p className="mt-1 text-muted-foreground text-xs leading-relaxed">
            {t(`welcome.welcome.${key}.body`)}
          </p>
        </li>
      ))}
    </ul>
  );
}

function PrivacyFacts() {
  const { t } = useTranslation();
  const facts: { key: "watchOnly" | "local" | "openSource"; icon: LucideIcon }[] = [
    { key: "watchOnly", icon: ShieldCheck },
    { key: "local", icon: HardDrive },
    { key: "openSource", icon: Code2 },
  ];
  return (
    <ul className="space-y-4">
      {facts.map(({ key, icon: Icon }) => (
        <li key={key} className="flex gap-4">
          <Icon className="mt-0.5 size-5 shrink-0 text-success" />
          <div>
            <p className="font-semibold text-sm">{t(`welcome.privacy.${key}.title`)}</p>
            <p className="mt-0.5 text-muted-foreground text-sm leading-relaxed">
              {t(`welcome.privacy.${key}.body`)}
            </p>
          </div>
        </li>
      ))}
    </ul>
  );
}

function PersonalizeControls() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const currency = usePreferences((s) => s.currency);
  const setCurrency = usePreferences((s) => s.setCurrency);
  return (
    <div className="divide-y divide-border rounded-2xl border bg-card px-5">
      <Row label={t("settings.language")}>
        <SegmentedControl
          label={t("settings.language")}
          options={SUPPORTED_LANGUAGES.map((l) => l.code)}
          labels={{ "pt-BR": "Português", en: "English" }}
          value={(i18n.resolvedLanguage ?? "en") as Language}
          onChange={(v) => i18n.changeLanguage(v)}
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
        />
      </Row>
      <Row label={t("settings.currency")}>
        <SegmentedControl
          label={t("settings.currency")}
          options={CURRENCIES}
          value={currency}
          onChange={setCurrency}
        />
      </Row>
    </div>
  );
}

function ModulesControls() {
  const { t } = useTranslation();
  const taxModuleEnabled = usePreferences((s) => s.taxModuleEnabled);
  const setTaxModuleEnabled = usePreferences((s) => s.setTaxModuleEnabled);
  return (
    <div className="space-y-3">
      <label
        htmlFor="welcome-tax"
        className={cn(
          "flex cursor-pointer items-start gap-4 rounded-2xl border-2 p-5 transition-colors",
          taxModuleEnabled ? "border-primary bg-primary/8" : "border-border bg-card",
        )}
      >
        <span className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-primary/12 text-primary">
          <Receipt className="size-5" />
        </span>
        <div className="flex-1">
          <p className="font-semibold">{t("settings.taxModule")}</p>
          <p className="mt-1 text-muted-foreground text-sm leading-relaxed">
            {t("welcome.modules.taxBody")}
          </p>
        </div>
        <Switch id="welcome-tax" checked={taxModuleEnabled} onCheckedChange={setTaxModuleEnabled} />
      </label>
      <p className="text-muted-foreground text-xs">{t("welcome.modules.later")}</p>
    </div>
  );
}

function ReadyActions({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const setDemoMode = usePreferences((s) => s.setDemoMode);
  const startTour = useTour((s) => s.start);

  const actions: {
    key: "wallet" | "tour" | "demo";
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
    <div className="space-y-3">
      {actions.map(({ key, icon: Icon, primary, run }) => (
        <TiltButton key={key} onClick={run} primary={primary}>
          <span
            className={cn(
              "flex size-11 shrink-0 items-center justify-center rounded-xl",
              primary ? "bg-primary-foreground/15" : "bg-muted",
            )}
          >
            <Icon className="size-5" />
          </span>
          <span className="flex-1">
            <span className="block font-semibold">{t(`welcome.ready.${key}.title`)}</span>
            <span
              className={cn(
                "block text-sm",
                primary ? "text-primary-foreground/80" : "text-muted-foreground",
              )}
            >
              {t(`welcome.ready.${key}.body`)}
            </span>
          </span>
          <ArrowRight className="size-5 transition-transform group-hover:translate-x-1" />
        </TiltButton>
      ))}
    </div>
  );
}

/** Card button that tilts toward the pointer. */
function TiltButton({
  primary,
  onClick,
  children,
}: {
  primary?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  const { contextSafe } = useGSAP({ scope: ref });

  const onMove = contextSafe((e: React.PointerEvent) => {
    if (!window.matchMedia(MOTION_OK).matches || !ref.current) return;
    const box = ref.current.getBoundingClientRect();
    const x = (e.clientX - box.left) / box.width - 0.5;
    const y = (e.clientY - box.top) / box.height - 0.5;
    gsap.to(ref.current, {
      rotateY: x * 8,
      rotateX: -y * 8,
      transformPerspective: 700,
      duration: 0.4,
      ease: "power2.out",
    });
  });
  const onLeave = contextSafe(() => {
    gsap.to(ref.current, { rotateX: 0, rotateY: 0, duration: 0.6, ease: "elastic.out(1, 0.5)" });
  });

  return (
    <button
      ref={ref}
      type="button"
      onClick={onClick}
      onPointerMove={onMove}
      onPointerLeave={onLeave}
      className={cn(
        "group flex w-full items-center gap-4 rounded-2xl p-5 text-left transition-colors",
        primary
          ? "bg-primary text-primary-foreground hover:bg-primary/90"
          : "border bg-card hover:bg-accent",
      )}
    >
      {children}
    </button>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-4">
      <span className="font-medium text-sm">{label}</span>
      {children}
    </div>
  );
}
