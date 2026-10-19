import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export const CURRENCIES = ["USD", "BRL", "EUR"] as const;
export type Currency = (typeof CURRENCIES)[number];

interface PreferencesState {
  currency: Currency;
  privacyMode: boolean;
  taxModuleEnabled: boolean;
  setCurrency: (currency: Currency) => void;
  togglePrivacyMode: () => void;
  setTaxModuleEnabled: (enabled: boolean) => void;
}

/**
 * UI-only preferences kept in localStorage. Anything that affects financial
 * calculations will move to the Rust settings table once the database lands.
 */
export const usePreferences = create<PreferencesState>()(
  persist(
    (set) => ({
      currency: "USD",
      privacyMode: false,
      taxModuleEnabled: false,
      setCurrency: (currency) => set({ currency }),
      togglePrivacyMode: () => set((s) => ({ privacyMode: !s.privacyMode })),
      setTaxModuleEnabled: (taxModuleEnabled) => set({ taxModuleEnabled }),
    }),
    {
      name: "chainfolio.preferences",
      storage: createJSONStorage(() => localStorage),
      version: 1,
    },
  ),
);
