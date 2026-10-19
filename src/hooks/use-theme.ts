import { useEffect, useSyncExternalStore } from "react";
import { usePreferences } from "@/stores/preferences";

const darkQuery = "(prefers-color-scheme: dark)";

function subscribeToSystemTheme(onChange: () => void) {
  const mql = window.matchMedia(darkQuery);
  mql.addEventListener("change", onChange);
  return () => mql.removeEventListener("change", onChange);
}

function useSystemPrefersDark() {
  return useSyncExternalStore(subscribeToSystemTheme, () => window.matchMedia(darkQuery).matches);
}

export function useTheme() {
  const theme = usePreferences((s) => s.theme);
  const setTheme = usePreferences((s) => s.setTheme);
  const systemDark = useSystemPrefersDark();
  const resolvedTheme: "dark" | "light" =
    theme === "system" ? (systemDark ? "dark" : "light") : theme;

  const toggleTheme = () => setTheme(resolvedTheme === "dark" ? "light" : "dark");

  return { theme, resolvedTheme, setTheme, toggleTheme };
}

/** Keeps the `dark` class on <html> in sync with the selected theme. */
export function useApplyTheme() {
  const { resolvedTheme } = useTheme();
  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle("dark", resolvedTheme === "dark");
    root.style.colorScheme = resolvedTheme;
  }, [resolvedTheme]);
}
