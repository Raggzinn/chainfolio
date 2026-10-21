import { KeyRound, Loader2, Lock, ShieldCheck, ShieldOff } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Panel } from "@/components/panel";
import { SegmentedControl } from "@/components/segmented-control";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { IS_TAURI } from "@/lib/platform";
import { usePreferences } from "@/stores/preferences";
import { useLock, useSecurityStatus, useSetMasterPassword } from "./queries";

const AUTO_LOCK = ["0", "5", "15", "30", "60"] as const;
type Mode = "set" | "change" | "remove";

/** Settings section: master password and auto-lock. */
export function SecurityPanel() {
  const { t } = useTranslation();
  const { data } = useSecurityStatus();
  const lock = useLock();
  const minutes = usePreferences((s) => s.autoLockMinutes);
  const setMinutes = usePreferences((s) => s.setAutoLockMinutes);
  const [mode, setMode] = useState<Mode | null>(null);
  const encrypted = data?.encrypted ?? false;

  return (
    <Panel title={t("security.title")} subtitle={t("security.subtitle")}>
      <div className="divide-y divide-border">
        <div className="flex items-center justify-between gap-6 pb-4">
          <div className="flex items-start gap-3">
            <span
              className={`flex size-9 shrink-0 items-center justify-center rounded-lg ${
                encrypted ? "bg-success/12 text-success" : "bg-muted text-muted-foreground"
              }`}
            >
              {encrypted ? <ShieldCheck className="size-4" /> : <ShieldOff className="size-4" />}
            </span>
            <div className="space-y-1">
              <p className="font-medium text-sm">
                {encrypted ? t("security.on") : t("security.off")}
              </p>
              <p className="max-w-lg text-muted-foreground text-sm">
                {encrypted ? t("security.onHint") : t("security.offHint")}
              </p>
            </div>
          </div>
          <div className="flex shrink-0 gap-2">
            {encrypted ? (
              <>
                <Button variant="ghost" onClick={() => lock.mutate()} className="rounded-lg">
                  <Lock />
                  {t("security.lockNow")}
                </Button>
                <Button variant="outline" onClick={() => setMode("change")} className="rounded-lg">
                  {t("security.change")}
                </Button>
                <Button variant="outline" onClick={() => setMode("remove")} className="rounded-lg">
                  {t("security.remove")}
                </Button>
              </>
            ) : (
              <Button disabled={!IS_TAURI} onClick={() => setMode("set")} className="rounded-lg">
                <KeyRound />
                {t("security.set")}
              </Button>
            )}
          </div>
        </div>
        <div className="flex items-center justify-between gap-6 pt-4">
          <div className="space-y-1">
            <p className="font-medium text-sm">{t("security.autoLock")}</p>
            <p className="max-w-lg text-muted-foreground text-sm">{t("security.autoLockHint")}</p>
          </div>
          <SegmentedControl
            label={t("security.autoLock")}
            options={AUTO_LOCK}
            labels={Object.fromEntries(
              AUTO_LOCK.map((m) => [m, m === "0" ? t("security.never") : `${m} min`]),
            )}
            value={String(minutes) as (typeof AUTO_LOCK)[number]}
            onChange={(v) => setMinutes(Number(v))}
          />
        </div>
      </div>
      <PasswordDialog mode={mode} onClose={() => setMode(null)} />
    </Panel>
  );
}

function PasswordDialog({ mode, onClose }: { mode: Mode | null; onClose: () => void }) {
  const { t } = useTranslation();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const save = useSetMasterPassword();

  const close = () => {
    setCurrent("");
    setNext("");
    setConfirm("");
    save.reset();
    onClose();
  };

  const needsCurrent = mode === "change" || mode === "remove";
  const needsNext = mode === "set" || mode === "change";
  const mismatch = needsNext && confirm.length > 0 && next !== confirm;
  const ready =
    (!needsCurrent || current.length > 0) && (!needsNext || (next.length > 0 && next === confirm));

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!mode || !ready || save.isPending) return;
    save.mutate(
      { current: needsCurrent ? current : null, next: needsNext ? next : null },
      {
        onSuccess: () => {
          toast.success(t(`security.${mode}Done`));
          close();
        },
      },
    );
  };

  return (
    <Dialog open={mode !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent className="surface-raised gap-5 rounded-2xl border-0 sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{mode && t(`security.${mode}Title`)}</DialogTitle>
          <DialogDescription>{mode && t(`security.${mode}Description`)}</DialogDescription>
        </DialogHeader>
        <form id="master-password" onSubmit={submit} className="space-y-4">
          {needsCurrent && (
            <Field id="current-password" label={t("security.current")}>
              <Input
                id="current-password"
                type="password"
                value={current}
                onChange={(e) => setCurrent(e.target.value)}
                autoFocus
                className="h-11 rounded-xl"
              />
            </Field>
          )}
          {needsNext && (
            <>
              <Field id="new-password" label={t("security.new")}>
                <Input
                  id="new-password"
                  type="password"
                  value={next}
                  onChange={(e) => setNext(e.target.value)}
                  autoFocus={!needsCurrent}
                  className="h-11 rounded-xl"
                />
              </Field>
              <Field id="confirm-password" label={t("security.confirm")}>
                <Input
                  id="confirm-password"
                  type="password"
                  value={confirm}
                  onChange={(e) => setConfirm(e.target.value)}
                  aria-invalid={mismatch}
                  className="h-11 rounded-xl"
                />
              </Field>
            </>
          )}
          {needsNext && (
            <p className="rounded-xl bg-muted px-3 py-2.5 text-muted-foreground text-xs leading-relaxed">
              {t("security.noRecovery")}
            </p>
          )}
          {(mismatch || save.error) && (
            <p role="alert" className="text-danger text-xs">
              {mismatch || !save.error ? t("security.mismatch") : t(`errors.${save.error.code}`)}
            </p>
          )}
        </form>
        <DialogFooter>
          <Button variant="ghost" onClick={close}>
            {t("wallets.cancel")}
          </Button>
          <Button
            type="submit"
            form="master-password"
            variant={mode === "remove" ? "destructive" : "default"}
            disabled={!ready || save.isPending}
            className="min-w-28"
          >
            {save.isPending && <Loader2 className="animate-spin" />}
            {mode && t(`security.${mode}Confirm`)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Field({ id, label, children }: { id: string; label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
    </div>
  );
}
