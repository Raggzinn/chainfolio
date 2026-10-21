import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";
import { AppLayout } from "@/components/layout/app-layout";
import { AlertsPage } from "@/pages/alerts";
import { PlaceholderPage } from "@/pages/placeholder-page";
import { PortfolioPage } from "@/pages/portfolio";
import { SettingsPage } from "@/pages/settings";
import { WalletsPage } from "@/pages/wallets";
import { usePreferences } from "@/stores/preferences";

const rootRoute = createRootRoute({ component: AppLayout });

const page = <TPath extends string>(path: TPath, component: () => React.ReactNode) =>
  createRoute({ getParentRoute: () => rootRoute, path, component });

const taxesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/taxes",
  beforeLoad: () => {
    if (!usePreferences.getState().taxModuleEnabled) throw redirect({ to: "/settings" });
  },
  component: () => <PlaceholderPage page="taxes" />,
});

const routeTree = rootRoute.addChildren([
  page("/", PortfolioPage),
  page("/assets", () => <PlaceholderPage page="assets" />),
  page("/transactions", () => <PlaceholderPage page="transactions" />),
  page("/defi", () => <PlaceholderPage page="defi" />),
  page("/analytics", () => <PlaceholderPage page="analytics" />),
  page("/risk", () => <PlaceholderPage page="risk" />),
  taxesRoute,
  page("/alerts", AlertsPage),
  page("/wallets", WalletsPage),
  page("/settings", SettingsPage),
]);

// Hash history keeps deep links working when the bundle is served from the Tauri asset protocol.
export const router = createRouter({ routeTree, history: createHashHistory() });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
