export type TourStepId =
  | "navigation"
  | "search"
  | "privacy"
  | "portfolio"
  | "filters"
  | "risk"
  | "mood"
  | "wallets"
  | "settings";

/**
 * Ordered steps of the product tour. Each one highlights the element marked
 * with `data-tour="<id>"`; steps whose target isn't on screen are skipped.
 */
export const TOUR_STEPS: TourStepId[] = [
  "navigation",
  "search",
  "privacy",
  "portfolio",
  "filters",
  "risk",
  "mood",
  "wallets",
  "settings",
];

export const tourTarget = (id: TourStepId) => `[data-tour="${id}"]`;
