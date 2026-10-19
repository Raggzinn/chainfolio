import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export const CURRENCIES = ["USD", "BRL", "EUR"] as const;
export type Currency = (typeof CURRENCIES)[number];

export const THEMES = ["dark", "light", "system"] as const;
export type Theme = (typeof THEMES)[number];

interface PreferencesState {
  theme: Theme;
  currency: Currency;
  privacyMode: boolean;
  taxModuleEnabled: boolean;
  demoMode: boolean;
  sidebarCollapsed: boolean;
  setTheme: (theme: Theme) => void;
  setCurrency: (currency: Currency) => void;
  togglePrivacyMode: () => void;
  setTaxModuleEnabled: (enabled: boolean) => void;
  setDemoMode: (enabled: boolean) => void;
  toggleSidebar: () => void;
}

/**
 * UI-only preferences kept in localStorage. Anything that affects financial
 * calculations will move to the Rust settings table once the database lands.
 */
export const usePreferences = create<PreferencesState>()(
  persist(
    (set) => ({
      theme: "dark",
      currency: "USD",
      privacyMode: false,
      taxModuleEnabled: false,
      demoMode: false,
      sidebarCollapsed: false,
      setTheme: (theme) => set({ theme }),
      setCurrency: (currency) => set({ currency }),
      togglePrivacyMode: () => set((s) => ({ privacyMode: !s.privacyMode })),
      setTaxModuleEnabled: (taxModuleEnabled) => set({ taxModuleEnabled }),
      setDemoMode: (demoMode) => set({ demoMode }),
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
    }),
    {
      name: "chainfolio.preferences",
      storage: createJSONStorage(() => localStorage),
      version: 1,
    },
  ),
);
