import { FlaskConical, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { MarketMoodCard } from "@/features/market/market-mood-card";
import { ActivityCard } from "@/features/portfolio/activity-card";
import { AllocationCard } from "@/features/portfolio/allocation-card";
import { HeroCard } from "@/features/portfolio/hero-card";
import { Onboarding } from "@/features/portfolio/onboarding";
import { RiskCard } from "@/features/portfolio/risk-card";
import { TopAssetsCard } from "@/features/portfolio/top-assets-card";
import { usePreferences } from "@/stores/preferences";

export function PortfolioPage() {
  const demoMode = usePreferences((s) => s.demoMode);
  if (!demoMode) return <Onboarding />;

  return (
    <>
      <DemoBanner />
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <HeroCard />
        <RiskCard />
      </div>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <TopAssetsCard />
        <MarketMoodCard />
      </div>
      <div className="grid gap-5 lg:grid-cols-2">
        <AllocationCard />
        <ActivityCard />
      </div>
    </>
  );
}

function DemoBanner() {
  const { t } = useTranslation();
  const setDemoMode = usePreferences((s) => s.setDemoMode);
  return (
    <div className="surface flex items-center gap-3 rounded-xl px-4 py-2.5 text-sm">
      <span className="flex size-6 items-center justify-center rounded-lg bg-chart-5/15 text-chart-5">
        <FlaskConical className="size-3.5" />
      </span>
      <span className="text-muted-foreground">{t("dashboard.demoBanner")}</span>
      <button
        type="button"
        onClick={() => setDemoMode(false)}
        className="ml-auto inline-flex items-center gap-1 rounded-lg px-2 py-1 font-medium text-xs transition-colors hover:bg-accent"
      >
        <X className="size-3.5" />
        {t("dashboard.exitDemo")}
      </button>
    </div>
  );
}
