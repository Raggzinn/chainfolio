import { TOUR_STEPS } from "@/features/onboarding/tour-steps";
import { usePreferences } from "./preferences";
import { useTour } from "./tour";

describe("tour store", () => {
  beforeEach(() => {
    useTour.setState({ active: false, step: 0, demoBefore: null });
    usePreferences.setState({ demoMode: false });
  });

  it("turns demo mode on while running and restores it at the end", () => {
    useTour.getState().start();
    expect(useTour.getState().active).toBe(true);
    expect(usePreferences.getState().demoMode).toBe(true);

    useTour.getState().stop();
    expect(useTour.getState().active).toBe(false);
    expect(usePreferences.getState().demoMode).toBe(false);
  });

  it("keeps demo mode on if it was already on", () => {
    usePreferences.setState({ demoMode: true });
    useTour.getState().start();
    useTour.getState().stop();
    expect(usePreferences.getState().demoMode).toBe(true);
  });

  it("walks forward and back, and finishes after the last step", () => {
    const tour = useTour.getState();
    tour.start();
    tour.prev();
    expect(useTour.getState().step).toBe(0);
    for (let i = 1; i < TOUR_STEPS.length; i++) useTour.getState().next();
    expect(useTour.getState().step).toBe(TOUR_STEPS.length - 1);
    useTour.getState().next();
    expect(useTour.getState().active).toBe(false);
  });
});
