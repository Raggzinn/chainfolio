import { useNavigate } from "@tanstack/react-router";
import { Check, Compass, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/layout/page-header";
import { Panel } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { SecurityPanel } from "@/features/security/security-panel";
import { UpdatesPanel } from "@/features/updates/updates-panel";
import { useTheme } from "@/hooks/use-theme";
import { type Language, SUPPORTED_LANGUAGES } from "@/i18n";
import { cn } from "@/lib/utils";
import { CURRENCIES, THEMES, type Theme, usePreferences } from "@/stores/preferences";
import { useTour } from "@/stores/tour";

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const { currency, setCurrency, taxModuleEnabled, setTaxModuleEnabled, demoMode, setDemoMode } =
    usePreferences();

  const themeLabels: Record<Theme, string> = {
    dark: t("settings.themeDark"),
    light: t("settings.themeLight"),
    system: t("settings.themeSystem"),
  };

  return (
    <>
      <PageHeader title={t("pages.settings.title")} description={t("pages.settings.description")} />

      <Panel title={t("settings.theme")} subtitle={t("settings.themeHint")}>
        <div className="grid grid-cols-3 gap-4">
          {THEMES.map((option) => (
            <button
              key={option}
              type="button"
              onClick={() => setTheme(option)}
              className="group space-y-2.5 text-left"
            >
              <ThemePreview variant={option} selected={theme === option} />
              <span
                className={cn(
                  "flex items-center gap-1.5 font-medium text-sm",
                  theme === option ? "text-foreground" : "text-muted-foreground",
                )}
              >
                {theme === option && <Check className="size-4 text-primary" />}
                {themeLabels[option]}
              </span>
            </button>
          ))}
        </div>
      </Panel>

      <Panel title={t("settings.general")}>
        <div className="divide-y divide-border">
          <SettingRow label={t("settings.language")} hint={t("settings.languageHint")}>
            <SegmentedControl
              label={t("settings.language")}
              options={SUPPORTED_LANGUAGES.map((l) => l.code)}
              labels={{ "pt-BR": "Português", en: "English" }}
              value={(i18n.resolvedLanguage ?? "en") as Language}
              onChange={(v) => i18n.changeLanguage(v)}
            />
          </SettingRow>
          <SettingRow label={t("settings.currency")} hint={t("settings.currencyHint")}>
            <SegmentedControl
              label={t("settings.currency")}
              options={CURRENCIES}
              value={currency}
              onChange={setCurrency}
            />
          </SettingRow>
        </div>
      </Panel>

      <Panel title={t("settings.modules")}>
        <div className="divide-y divide-border">
          <SettingRow
            id="tax-module"
            label={t("settings.taxModule")}
            hint={t("settings.taxModuleHint")}
          >
            <Switch
              id="tax-module"
              checked={taxModuleEnabled}
              onCheckedChange={setTaxModuleEnabled}
            />
          </SettingRow>
          <SettingRow
            id="demo-mode"
            label={t("settings.demoMode")}
            hint={t("settings.demoModeHint")}
          >
            <Switch id="demo-mode" checked={demoMode} onCheckedChange={setDemoMode} />
          </SettingRow>
        </div>
      </Panel>

      <SecurityPanel />

      <UpdatesPanel />

      <HelpPanel />
    </>
  );
}

function SettingRow({
  id,
  label,
  hint,
  children,
}: {
  id?: string;
  label: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 py-4 first:pt-0 last:pb-0">
      <div className="space-y-1">
        <Label htmlFor={id} className="text-sm">
          {label}
        </Label>
        <p className="max-w-lg text-muted-foreground text-sm">{hint}</p>
      </div>
      {children}
    </div>
  );
}

/** Miniature app window drawn in the given theme. */
function ThemePreview({ variant, selected }: { variant: Theme; selected: boolean }) {
  const half = variant === "system";
  return (
    <div
      className={cn(
        "relative aspect-[16/10] overflow-hidden rounded-xl ring-1 ring-border transition-all duration-300 group-hover:-translate-y-0.5",
        selected && "ring-2 ring-primary",
      )}
    >
      <MiniWindow dark={variant !== "light"} />
      {half && (
        <div
          className="absolute inset-0"
          style={{ clipPath: "polygon(55% 0, 100% 0, 100% 100%, 45% 100%)" }}
        >
          <MiniWindow dark={false} />
        </div>
      )}
    </div>
  );
}

function MiniWindow({ dark }: { dark: boolean }) {
  const bg = dark ? "#141517" : "#f2f2f4";
  const panel = dark ? "#1c1d20" : "#fafafb";
  const line = dark ? "#2b2c30" : "#e1e1e5";
  return (
    <div className="absolute inset-0 flex gap-1.5 p-2" style={{ background: bg }}>
      <div className="w-1/4 space-y-1.5 rounded-md p-1.5" style={{ background: panel }}>
        <div className="h-1.5 w-3/4 rounded-full bg-primary" />
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="h-1 rounded-full" style={{ background: line }} />
        ))}
      </div>
      <div className="flex flex-1 flex-col gap-1.5">
        <div className="flex-1 rounded-md p-1.5" style={{ background: panel }}>
          <svg
            viewBox="0 0 100 40"
            className="h-full w-full"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <path
              d="M0 32 L15 28 L30 30 L45 20 L60 22 L75 12 L100 6"
              fill="none"
              stroke="#1E6BFF"
              strokeWidth="2.5"
            />
          </svg>
        </div>
        <div className="flex h-1/3 gap-1.5">
          <div className="flex-1 rounded-md" style={{ background: panel }} />
          <div className="flex-1 rounded-md" style={{ background: panel }} />
        </div>
      </div>
    </div>
  );
}

function HelpPanel() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const startTour = useTour((s) => s.start);
  const setWelcomeCompleted = usePreferences((s) => s.setWelcomeCompleted);
  return (
    <Panel title={t("settings.help")}>
      <div className="grid gap-3 sm:grid-cols-2">
        <HelpAction
          icon={Compass}
          title={t("tour.start")}
          body={t("settings.tourHint")}
          onClick={() => {
            navigate({ to: "/" });
            startTour();
          }}
        />
        <HelpAction
          icon={Sparkles}
          title={t("welcome.show")}
          body={t("settings.welcomeHint")}
          onClick={() => setWelcomeCompleted(false)}
        />
      </div>
    </Panel>
  );
}

function HelpAction({
  icon: Icon,
  title,
  body,
  onClick,
}: {
  icon: typeof Compass;
  title: string;
  body: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex items-start gap-3 rounded-xl bg-muted p-4 text-left transition-colors hover:bg-accent"
    >
      <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/12 text-primary">
        <Icon className="size-4" />
      </span>
      <span>
        <span className="block font-semibold text-sm">{title}</span>
        <span className="mt-0.5 block text-muted-foreground text-sm">{body}</span>
      </span>
    </button>
  );
}
