import { Loader2, LockKeyhole } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand/brand-mark";
import { WindowControls } from "@/components/layout/window-controls";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { gsap, MOTION_OK } from "@/lib/gsap";
import { IS_TAURI } from "@/lib/platform";
import { usePreferences } from "@/stores/preferences";
import { useLock, useSecurityStatus, useUnlock } from "./queries";
import { useIdleTimer } from "./use-idle-lock";

/**
 * Shows the lock screen instead of the app while the encrypted database is
 * closed, and closes it again after the configured idle time.
 */
export function SecurityGate({ children }: { children: React.ReactNode }) {
  const { data, isPending } = useSecurityStatus();
  const lock = useLock();
  const minutes = usePreferences((s) => s.autoLockMinutes);
  const unlocked = Boolean(data?.encrypted && !data.locked);
  const { mutate } = lock;
  const onIdle = useCallback(() => mutate(), [mutate]);
  useIdleTimer(unlocked ? minutes : 0, onIdle);

  if (IS_TAURI && isPending) return null;
  if (data?.locked) return <LockScreen />;
  return children;
}

function LockScreen() {
  const { t } = useTranslation();
  const [password, setPassword] = useState("");
  const unlock = useUnlock();
  const card = useRef<HTMLFormElement>(null);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!password || unlock.isPending) return;
    unlock.mutate(password, {
      onError: () => {
        setPassword("");
        if (window.matchMedia(MOTION_OK).matches) {
          gsap.fromTo(
            card.current,
            { x: -10 },
            { x: 0, duration: 0.5, ease: "elastic.out(1, 0.3)" },
          );
        }
      },
    });
  };

  const error = unlock.error;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-background">
      <header data-tauri-drag-region className="flex h-12 shrink-0 items-center justify-end pr-4">
        <WindowControls />
      </header>
      <div className="flex flex-1 items-center justify-center p-6">
        <form
          ref={card}
          onSubmit={submit}
          className="surface-raised w-full max-w-sm space-y-6 rounded-2xl p-8 text-center"
        >
          <div className="flex flex-col items-center gap-4">
            <BrandMark className="size-14" />
            <div className="space-y-1.5">
              <h1 className="flex items-center justify-center gap-2 font-semibold text-xl tracking-tight">
                <LockKeyhole className="size-5 text-muted-foreground" />
                {t("security.lockedTitle")}
              </h1>
              <p className="text-muted-foreground text-sm">{t("security.lockedHint")}</p>
            </div>
          </div>
          <div className="space-y-2 text-left">
            <Input
              type="password"
              value={password}
              onChange={(e) => {
                setPassword(e.target.value);
                unlock.reset();
              }}
              placeholder={t("security.password")}
              aria-label={t("security.password")}
              aria-invalid={Boolean(error)}
              autoFocus
              className="h-11 rounded-xl"
            />
            {error && (
              <p role="alert" className="text-danger text-xs">
                {t(`errors.${error.code}`)}
              </p>
            )}
          </div>
          <Button
            type="submit"
            disabled={!password || unlock.isPending}
            className="h-11 w-full rounded-xl"
          >
            {unlock.isPending && <Loader2 className="animate-spin" />}
            {t("security.unlock")}
          </Button>
        </form>
      </div>
    </div>
  );
}
