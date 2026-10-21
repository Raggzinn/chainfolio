import { useEffect } from "react";

const ACTIVITY = ["pointerdown", "pointermove", "keydown", "wheel", "touchstart"] as const;

/**
 * Calls `onIdle` once nobody has touched the window for `minutes`.
 * `minutes <= 0` disables the timer.
 */
export function useIdleTimer(minutes: number, onIdle: () => void) {
  useEffect(() => {
    if (minutes <= 0) return;
    const ms = minutes * 60_000;
    let timer = window.setTimeout(onIdle, ms);
    const reset = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(onIdle, ms);
    };
    for (const event of ACTIVITY) window.addEventListener(event, reset, { passive: true });
    return () => {
      window.clearTimeout(timer);
      for (const event of ACTIVITY) window.removeEventListener(event, reset);
    };
  }, [minutes, onIdle]);
}
