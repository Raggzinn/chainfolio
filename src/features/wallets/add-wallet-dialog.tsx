import { CircleAlert, Loader2, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
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
import { type Detection, toApiError } from "@/lib/api";
import { CHAINS, FAMILY_SYMBOL } from "@/lib/chains";
import { useAddWallet, useDetectAddress } from "./queries";

export function AddWalletDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const [input, setInput] = useState("");
  const [label, setLabel] = useState("");
  const detection = useDetectAddress(input);
  const add = useAddWallet();

  const reset = (next: boolean) => {
    if (!next) {
      setInput("");
      setLabel("");
      add.reset();
    }
    onOpenChange(next);
  };

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!detection.data || add.isPending) return;
    add.mutate(
      { input, label },
      {
        onSuccess: (wallet) => {
          toast.success(t("wallets.added", { label: wallet.label }), {
            description: t("wallets.loadingBalances"),
          });
          reset(false);
        },
      },
    );
  };

  const typed = input.trim().length > 0;
  const error = add.error ?? (typed ? detection.error : null);

  return (
    <Dialog open={open} onOpenChange={reset}>
      <DialogContent className="surface-raised gap-5 rounded-2xl border-0 sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("wallets.addTitle")}</DialogTitle>
          <DialogDescription>{t("wallets.addDescription")}</DialogDescription>
        </DialogHeader>

        <form id="add-wallet" onSubmit={submit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="wallet-input">{t("wallets.addressLabel")}</Label>
            <Input
              id="wallet-input"
              value={input}
              onChange={(e) => {
                setInput(e.target.value);
                add.reset();
              }}
              placeholder="0x…, vitalik.eth, bc1…, zpub…, .sol, T…, r…"
              autoComplete="off"
              autoCorrect="off"
              spellCheck={false}
              autoFocus
              aria-invalid={Boolean(error)}
              className="h-11 rounded-xl font-mono text-sm"
            />
            <DetectionHint detection={typed ? detection.data : undefined} error={error} />
          </div>

          <div className="space-y-2">
            <Label htmlFor="wallet-label">{t("wallets.nameLabel")}</Label>
            <Input
              id="wallet-label"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder={t("wallets.namePlaceholder")}
              maxLength={60}
              className="h-11 rounded-xl"
            />
          </div>

          <p className="flex items-start gap-2 rounded-xl bg-muted px-3 py-2.5 text-muted-foreground text-xs leading-relaxed">
            <ShieldCheck className="mt-px size-4 shrink-0 text-success" />
            {t("wallets.watchOnlyNote")}
          </p>
        </form>

        <DialogFooter>
          <Button variant="ghost" onClick={() => reset(false)}>
            {t("wallets.cancel")}
          </Button>
          <Button
            type="submit"
            form="add-wallet"
            disabled={!detection.data || add.isPending}
            className="min-w-28"
          >
            {add.isPending && <Loader2 className="animate-spin" />}
            {add.isPending && detection.data && detection.data.kind !== "address"
              ? t("wallets.resolving")
              : t("topbar.addWallet")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function DetectionHint({ detection, error }: { detection?: Detection; error: unknown }) {
  const { t } = useTranslation();

  if (error) {
    const { code } = toApiError(error);
    return (
      <p role="alert" className="flex items-center gap-1.5 text-danger text-xs">
        <CircleAlert className="size-3.5" />
        {t(`errors.${code}`)}
      </p>
    );
  }
  if (!detection) {
    return <p className="text-muted-foreground text-xs">{t("wallets.supported")}</p>;
  }

  return (
    <div className="flex flex-wrap items-center gap-2 text-xs">
      <span className="inline-flex items-center gap-1.5 rounded-full bg-success/12 py-0.5 pr-2.5 pl-0.5 font-medium text-success">
        <TokenIcon symbol={FAMILY_SYMBOL[detection.family]} size={18} />
        {t(`wallets.kinds.${detection.kind}`, {
          family: t(`wallets.families.${detection.family}`),
        })}
      </span>
      <span className="text-muted-foreground">
        {detection.chains.length > 1
          ? t("wallets.scansChains", {
              count: detection.chains.length,
              chains: detection.chains.map((c) => CHAINS[c].name).join(", "),
            })
          : CHAINS[detection.chains[0] ?? "ethereum"].name}
      </span>
    </div>
  );
}
