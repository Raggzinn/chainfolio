import { Download, RefreshCw, Sparkles, X } from "lucide-react";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { gsap, MOTION_OK, useGSAP } from "@/lib/gsap";
import { usePreferences } from "@/stores/preferences";
import { useUpdater } from "@/stores/updater";

const FIRST_CHECK_DELAY_MS = 8_000;
const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

/** Checks for releases in the background and offers the update in a corner card. */
export function UpdateBanner() {
  const { t } = useTranslation();
  const autoCheck = usePreferences((s) => s.autoCheckUpdates);
  const { status, update, progress, dismissed, check, install, dismiss } = useUpdater();
  const card = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!autoCheck) return;
    const first = window.setTimeout(() => check({ silent: true }), FIRST_CHECK_DELAY_MS);
    const every = window.setInterval(() => check({ silent: true }), CHECK_INTERVAL_MS);
    return () => {
      window.clearTimeout(first);
      window.clearInterval(every);
    };
  }, [autoCheck, check]);

  const visible =
    !dismissed && (status === "available" || status === "downloading" || status === "ready");

  useGSAP(
    () => {
      if (!visible) return;
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap.from(card.current, {
          y: 24,
          autoAlpha: 0,
          scale: 0.96,
          duration: 0.6,
          ease: "expo.out",
        });
      });
      return () => mm.revert();
    },
    { dependencies: [visible] },
  );

  if (!visible || !update) return null;
  const busy = status === "downloading" || status === "ready";

  return (
    <div
      ref={card}
      role="status"
      className="surface-raised fixed right-5 bottom-5 z-40 w-[340px] rounded-2xl p-4"
    >
      <div className="flex items-start gap-3">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-primary/12 text-primary">
          <Sparkles className="size-4" />
        </span>
        <div className="min-w-0 flex-1">
          <p className="font-semibold text-sm">
            {t("updates.available", { version: update.version })}
          </p>
          <p className="mt-0.5 line-clamp-2 text-muted-foreground text-xs">
            {update.body?.trim() || t("updates.availableHint")}
          </p>
        </div>
        {!busy && (
          <button
            type="button"
            onClick={dismiss}
            aria-label={t("updates.later")}
            className="-mt-1 -mr-1 rounded-md p-1 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <X className="size-4" />
          </button>
        )}
      </div>

      {busy ? (
        <div className="mt-3 space-y-1.5">
          <div className="h-1.5 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-[width] duration-200"
              style={{ width: `${Math.round((progress ?? 0.1) * 100)}%` }}
            />
          </div>
          <p className="flex items-center gap-1.5 text-muted-foreground text-xs">
            <RefreshCw className="size-3 animate-spin" />
            {status === "ready" ? t("updates.restarting") : t("updates.downloading")}
          </p>
        </div>
      ) : (
        <div className="mt-3 flex justify-end gap-2">
          <button
            type="button"
            onClick={dismiss}
            className="rounded-lg px-3 py-1.5 font-medium text-sm transition-colors hover:bg-accent"
          >
            {t("updates.later")}
          </button>
          <button
            type="button"
            onClick={install}
            className="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 font-semibold text-primary-foreground text-sm transition-colors hover:bg-primary/90"
          >
            <Download className="size-4" />
            {t("updates.install")}
          </button>
        </div>
      )}
    </div>
  );
}
