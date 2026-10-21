import { FlaskConical, X } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import { MarketMoodCard } from "@/features/market/market-mood-card";
import { ActivityCard } from "@/features/portfolio/activity-card";
import { AllocationCard } from "@/features/portfolio/allocation-card";
import { HeroCard } from "@/features/portfolio/hero-card";
import { LiveHeroCard } from "@/features/portfolio/live-hero-card";
import { demoAssetRows, liveAssetRows } from "@/features/portfolio/model";
import { Onboarding } from "@/features/portfolio/onboarding";
import { RiskCard } from "@/features/portfolio/risk-card";
import { TopAssetsCard } from "@/features/portfolio/top-assets-card";
import { usePortfolio } from "@/features/wallets/queries";
import type { Portfolio } from "@/lib/api";
import { usePreferences } from "@/stores/preferences";

const DEMO_ROWS = demoAssetRows();

export function PortfolioPage() {
  const demoMode = usePreferences((s) => s.demoMode);
  const { data, isPending, fetchStatus } = usePortfolio();

  if (demoMode) return <DemoDashboard />;
  // `isPending` stays true for a disabled query (browser preview), so check it is running.
  if (isPending && fetchStatus === "fetching") return <DashboardSkeleton />;
  if (!data || data.wallets.length === 0) return <Onboarding />;
  return <LiveDashboard portfolio={data} />;
}

function LiveDashboard({ portfolio }: { portfolio: Portfolio }) {
  const rows = useMemo(() => liveAssetRows(portfolio), [portfolio]);
  return (
    <>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <LiveHeroCard portfolio={portfolio} />
        <MarketMoodCard />
      </div>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_420px]">
        <TopAssetsCard rows={rows} />
        <AllocationCard rows={rows} />
      </div>
    </>
  );
}

function DemoDashboard() {
  return (
    <>
      <DemoBanner />
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <HeroCard />
        <RiskCard />
      </div>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <TopAssetsCard rows={DEMO_ROWS} />
        <MarketMoodCard />
      </div>
      <div className="grid gap-5 lg:grid-cols-2">
        <AllocationCard rows={DEMO_ROWS} />
        <ActivityCard />
      </div>
    </>
  );
}

function DashboardSkeleton() {
  return (
    <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
      <Skeleton className="h-[360px] rounded-3xl" />
      <Skeleton className="h-[360px] rounded-2xl" />
    </div>
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
