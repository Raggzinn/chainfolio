import {
  ArrowLeftRight,
  Bell,
  ChartColumn,
  Coins,
  Layers,
  LayoutDashboard,
  type LucideIcon,
  Receipt,
  Settings,
  ShieldCheck,
  Wallet,
} from "lucide-react";

export type NavKey =
  | "portfolio"
  | "assets"
  | "transactions"
  | "analytics"
  | "defi"
  | "risk"
  | "taxes"
  | "alerts"
  | "wallets"
  | "settings";

export interface NavItem {
  key: NavKey;
  to: string;
  icon: LucideIcon;
  /** Only shown when the optional tax module is enabled. */
  requiresTaxModule?: boolean;
}

export interface NavGroup {
  label: "overview" | "insights" | "manage";
  items: NavItem[];
}

export const NAV_GROUPS: NavGroup[] = [
  {
    label: "overview",
    items: [
      { key: "portfolio", to: "/", icon: LayoutDashboard },
      { key: "assets", to: "/assets", icon: Coins },
      { key: "transactions", to: "/transactions", icon: ArrowLeftRight },
      { key: "defi", to: "/defi", icon: Layers },
    ],
  },
  {
    label: "insights",
    items: [
      { key: "analytics", to: "/analytics", icon: ChartColumn },
      { key: "risk", to: "/risk", icon: ShieldCheck },
      { key: "taxes", to: "/taxes", icon: Receipt, requiresTaxModule: true },
      { key: "alerts", to: "/alerts", icon: Bell },
    ],
  },
  {
    label: "manage",
    items: [
      { key: "wallets", to: "/wallets", icon: Wallet },
      { key: "settings", to: "/settings", icon: Settings },
    ],
  },
];

export function visibleNavGroups(taxModuleEnabled: boolean): NavGroup[] {
  return NAV_GROUPS.map((group) => ({
    ...group,
    items: group.items.filter((item) => !item.requiresTaxModule || taxModuleEnabled),
  }));
}
