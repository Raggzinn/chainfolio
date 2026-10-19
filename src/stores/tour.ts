import { create } from "zustand";
import { TOUR_STEPS } from "@/features/onboarding/tour-steps";
import { usePreferences } from "./preferences";

interface TourState {
  active: boolean;
  step: number;
  /** Demo mode value before the tour switched it on, restored when it ends. */
  demoBefore: boolean | null;
  start: () => void;
  next: () => void;
  prev: () => void;
  stop: () => void;
}

/**
 * The tour runs on the demo dashboard so every highlighted widget has data;
 * the previous demo mode is restored when it ends.
 */
export const useTour = create<TourState>()((set, get) => ({
  active: false,
  step: 0,
  demoBefore: null,
  start: () => {
    const prefs = usePreferences.getState();
    set({ active: true, step: 0, demoBefore: prefs.demoMode });
    if (!prefs.demoMode) prefs.setDemoMode(true);
  },
  next: () => {
    const { step } = get();
    if (step >= TOUR_STEPS.length - 1) get().stop();
    else set({ step: step + 1 });
  },
  prev: () => set((s) => ({ step: Math.max(0, s.step - 1) })),
  stop: () => {
    const { demoBefore } = get();
    if (demoBefore === false) usePreferences.getState().setDemoMode(false);
    set({ active: false, step: 0, demoBefore: null });
  },
}));
