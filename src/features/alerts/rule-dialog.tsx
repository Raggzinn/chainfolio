import { Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { SegmentedControl } from "@/components/segmented-control";
import { TokenIcon } from "@/components/token-icon";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { usePortfolio } from "@/features/wallets/queries";
import type { AlertCondition, AlertKind, AlertRule, Direction } from "@/lib/api";
import { formatUsd } from "./describe";
import { useSaveAlertRule, useWatchableAssets } from "./queries";

const KINDS = ["priceAbove", "priceBelow", "portfolioChange", "balanceChange", "depeg"] as const;
const DIRECTIONS = ["up", "down", "either"] as const;
const ANY_WALLET = "any";

/** Editable form state; numbers stay strings until saved. */
export interface Draft {
  kind: AlertKind;
  priceKey: string;
  amount: string;
  percent: string;
  direction: Direction;
  wallet: string;
}

export const EMPTY: Draft = {
  kind: "priceAbove",
  priceKey: "",
  amount: "",
  percent: "",
  direction: "either",
  wallet: ANY_WALLET,
};

/** Percent typed by the user -> ratio string for the backend ("5" -> "0.05"). */
export function percentToRatio(percent: string): string {
  return String(Number(percent.replace(",", ".")) / 100);
}

function fromRule(rule: AlertRule): Draft {
  const c = rule.condition;
  const percent = (ratio: string) => String(Number((Number(ratio) * 100).toFixed(4)));
  switch (c.kind) {
    case "priceAbove":
    case "priceBelow":
      return { ...EMPTY, kind: c.kind, priceKey: c.priceKey, amount: c.price };
    case "portfolioChange":
      return { ...EMPTY, kind: c.kind, direction: c.direction, percent: percent(c.ratio) };
    case "balanceChange":
      return {
        ...EMPTY,
        kind: c.kind,
        direction: c.direction,
        amount: c.usd,
        wallet: c.walletId === null ? ANY_WALLET : String(c.walletId),
      };
    case "depeg":
      return { ...EMPTY, kind: c.kind, percent: percent(c.ratio) };
  }
}

/** Builds the condition, or `null` while the form is incomplete. */
export function toCondition(
  draft: Draft,
  symbolOf: (priceKey: string) => string | undefined,
): AlertCondition | null {
  const amount = Number(draft.amount.replace(",", "."));
  const percent = Number(draft.percent.replace(",", "."));
  const amountOk = draft.amount.trim() !== "" && Number.isFinite(amount) && amount > 0;
  const percentOk = draft.percent.trim() !== "" && percent > 0 && percent < 100;
  const usd = String(amount);
  switch (draft.kind) {
    case "priceAbove":
    case "priceBelow": {
      const symbol = symbolOf(draft.priceKey);
      if (!symbol || !amountOk) return null;
      return { kind: draft.kind, priceKey: draft.priceKey, symbol, price: usd };
    }
    case "portfolioChange":
      if (!percentOk) return null;
      return {
        kind: "portfolioChange",
        direction: draft.direction,
        ratio: percentToRatio(draft.percent),
      };
    case "balanceChange":
      if (!amountOk) return null;
      return {
        kind: "balanceChange",
        walletId: draft.wallet === ANY_WALLET ? null : Number(draft.wallet),
        direction: draft.direction,
        usd,
      };
    case "depeg":
      if (!percentOk) return null;
      return { kind: "depeg", ratio: percentToRatio(draft.percent) };
  }
}

export function RuleDialog({
  open,
  rule,
  onOpenChange,
}: {
  open: boolean;
  /** Rule being edited; `null` creates a new one. */
  rule: AlertRule | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const [draft, setDraft] = useState<Draft>(EMPTY);
  const assets = useWatchableAssets(open);
  const { data: portfolio } = usePortfolio();
  const save = useSaveAlertRule();
  const { reset } = save;

  // Start from the rule (or a blank form) every time the dialog opens.
  useEffect(() => {
    if (open) {
      setDraft(rule ? fromRule(rule) : EMPTY);
      reset();
    }
  }, [open, rule, reset]);

  const set = (patch: Partial<Draft>) => setDraft((d) => ({ ...d, ...patch }));
  const asset = assets.data?.find((a) => a.priceKey === draft.priceKey);
  const condition = toCondition(draft, (key) => {
    const found = assets.data?.find((a) => a.priceKey === key);
    if (found) return found.symbol;
    // Keep the stored symbol when editing an asset that's no longer offered.
    const c = rule?.condition;
    return c && "priceKey" in c && c.priceKey === key ? c.symbol : undefined;
  });

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!condition || save.isPending) return;
    save.mutate(
      { id: rule?.id, condition },
      {
        onSuccess: () => {
          toast.success(rule ? t("alerts.updated") : t("alerts.created"));
          onOpenChange(false);
        },
      },
    );
  };

  const kindLabels = Object.fromEntries(KINDS.map((k) => [k, t(`alerts.kinds.${k}`)]));
  const directionLabels = {
    up: t("alerts.directions.up"),
    down: t("alerts.directions.down"),
    either: t("alerts.directions.either"),
  };
  const isPrice = draft.kind === "priceAbove" || draft.kind === "priceBelow";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="surface-raised gap-5 rounded-2xl border-0 sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{rule ? t("alerts.editTitle") : t("alerts.newTitle")}</DialogTitle>
          <DialogDescription>{t("alerts.dialogDescription")}</DialogDescription>
        </DialogHeader>

        <form id="alert-rule" onSubmit={submit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="alert-kind">{t("alerts.kind")}</Label>
            <Select value={draft.kind} onValueChange={(kind) => set({ kind: kind as AlertKind })}>
              <SelectTrigger id="alert-kind" className="h-11 w-full rounded-xl">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KINDS.map((k) => (
                  <SelectItem key={k} value={k}>
                    {kindLabels[k]}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-muted-foreground text-xs">{t(`alerts.kindHints.${draft.kind}`)}</p>
          </div>

          {isPrice && (
            <div className="space-y-2">
              <Label htmlFor="alert-asset">{t("alerts.asset")}</Label>
              <Select value={draft.priceKey} onValueChange={(priceKey) => set({ priceKey })}>
                <SelectTrigger id="alert-asset" className="h-11 w-full rounded-xl">
                  <SelectValue placeholder={t("alerts.pickAsset")} />
                </SelectTrigger>
                <SelectContent className="max-h-72">
                  {assets.data?.map((a) => (
                    <SelectItem key={a.priceKey} value={a.priceKey}>
                      <TokenIcon symbol={a.symbol} size={18} />
                      <span className="font-medium">{a.symbol}</span>
                      <span className="text-muted-foreground">{a.name}</span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {(draft.kind === "portfolioChange" || draft.kind === "balanceChange") && (
            <div className="space-y-2">
              <Label>{t("alerts.direction")}</Label>
              <SegmentedControl
                label={t("alerts.direction")}
                options={DIRECTIONS}
                labels={directionLabels}
                value={draft.direction}
                onChange={(direction) => set({ direction })}
              />
            </div>
          )}

          {draft.kind === "balanceChange" && (
            <div className="space-y-2">
              <Label htmlFor="alert-wallet">{t("alerts.wallet")}</Label>
              <Select value={draft.wallet} onValueChange={(wallet) => set({ wallet })}>
                <SelectTrigger id="alert-wallet" className="h-11 w-full rounded-xl">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={ANY_WALLET}>{t("alerts.anyWallet")}</SelectItem>
                  {portfolio?.wallets.map((w) => (
                    <SelectItem key={w.id} value={String(w.id)}>
                      {w.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {(isPrice || draft.kind === "balanceChange") && (
            <div className="space-y-2">
              <Label htmlFor="alert-amount">
                {isPrice ? t("alerts.targetPrice") : t("alerts.minimumMove")}
              </Label>
              <Input
                id="alert-amount"
                inputMode="decimal"
                value={draft.amount}
                onChange={(e) => set({ amount: e.target.value })}
                placeholder="0.00"
                className="h-11 rounded-xl font-mono tabular-nums"
              />
              {isPrice && asset?.priceUsd && (
                <p className="text-muted-foreground text-xs">
                  {t("alerts.currentPrice", { price: formatUsd(Number(asset.priceUsd), locale) })}
                </p>
              )}
            </div>
          )}

          {(draft.kind === "portfolioChange" || draft.kind === "depeg") && (
            <div className="space-y-2">
              <Label htmlFor="alert-percent">{t("alerts.threshold")}</Label>
              <div className="relative">
                <Input
                  id="alert-percent"
                  inputMode="decimal"
                  value={draft.percent}
                  onChange={(e) => set({ percent: e.target.value })}
                  placeholder={draft.kind === "depeg" ? "1" : "10"}
                  className="h-11 rounded-xl pr-9 font-mono tabular-nums"
                />
                <span className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-muted-foreground text-sm">
                  %
                </span>
              </div>
            </div>
          )}

          {save.error && (
            <p role="alert" className="text-danger text-xs">
              {t(`errors.${save.error.code}`)}
            </p>
          )}
        </form>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {t("wallets.cancel")}
          </Button>
          <Button
            type="submit"
            form="alert-rule"
            disabled={!condition || save.isPending}
            className="min-w-28"
          >
            {save.isPending && <Loader2 className="animate-spin" />}
            {rule ? t("wallets.save") : t("alerts.create")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
