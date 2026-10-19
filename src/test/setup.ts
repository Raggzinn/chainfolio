import "@testing-library/jest-dom/vitest";

// jsdom has no matchMedia. Report a full-motion, dark-preferring environment so
// GSAP animations actually run in component tests.
if (!window.matchMedia) {
  window.matchMedia = (query: string) =>
    ({
      matches: query.includes("no-preference") || query.includes("dark"),
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList;
}
