import { Check, Copy, ExternalLink, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { TokenIcon } from "@/components/token-icon";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useMoney } from "@/hooks/use-money";
import { TX_KINDS, type TxItem, type TxKind } from "@/lib/api";
import { CHAINS, shortAddress, tokenColor } from "@/lib/chains";
import { openExternal, txUrl } from "@/lib/explorers";
import { cn } from "@/lib/utils";
import { useUpdateTransaction } from "./queries";
import { KIND_STYLE } from "./tx-kind";
import { useTxTitle } from "./tx-row";

export function TxDetailSheet({ tx, onClose }: { tx: TxItem | null; onClose: () => void }) {
  return (
    <Sheet open={tx !== null} onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="w-full gap-0 overflow-y-auto sm:max-w-md">
        {tx && <Detail key={`${tx.id}-${tx.walletId}`} tx={tx} onClose={onClose} />}
      </SheetContent>
    </Sheet>
  );
}

function Detail({ tx, onClose }: { tx: TxItem; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  const { format, hidden } = useMoney();
  const title = useTxTitle();
  const update = useUpdateTransaction();
  const [kind, setKind] = useState<TxKind>(tx.kind);
  const [note, setNote] = useState(tx.note ?? "");
  const [copied, setCopied] = useState(false);
  const { icon: Icon, tone } = KIND_STYLE[tx.kind];
  const date = new Intl.DateTimeFormat(i18n.resolvedLanguage, {
    dateStyle: "long",
    timeStyle: "short",
  });
  const amount = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumFractionDigits: 8 });
  const dirty = kind !== tx.kind || note.trim() !== (tx.note ?? "");

  useEffect(() => {
    if (!copied) return;
    const id = window.setTimeout(() => setCopied(false), 1500);
    return () => window.clearTimeout(id);
  }, [copied]);

  const save = () => {
    update.mutate(
      {
        txId: tx.id,
        walletId: tx.walletId,
        kind: kind === tx.autoKind ? null : kind,
        note: note.trim() || null,
      },
      {
        onSuccess: () => {
          toast.success(t("transactions.detail.saved"));
          onClose();
        },
        onError: (e) => toast.error(e.message),
      },
    );
  };

  return (
    <>
      <SheetHeader className="border-b p-6">
        <span className={cn("mb-2 flex size-11 items-center justify-center rounded-xl", tone)}>
          <Icon className="size-5" strokeWidth={2.2} />
        </span>
        <SheetTitle className="text-lg">{title(tx)}</SheetTitle>
        <SheetDescription>{date.format(tx.timestamp)}</SheetDescription>
      </SheetHeader>

      <div className="space-y-6 p-6">
        <section className="space-y-2">
          {tx.legs.map((leg) => {
            const n = Number(leg.amount);
            return (
              <div
                key={leg.asset.id}
                className="flex items-center gap-3 rounded-xl bg-muted px-3 py-2.5"
              >
                <TokenIcon
                  symbol={leg.asset.symbol}
                  color={tokenColor(leg.asset.symbol, leg.asset.chain)}
                  src={leg.asset.logoUrl}
                  size={32}
                />
                <div className="min-w-0 flex-1">
                  <p className={cn("tabular font-semibold text-sm", n > 0 && "text-success")}>
                    {hidden ? "•••" : `${n > 0 ? "+" : "−"}${amount.format(Math.abs(n))}`}{" "}
                    {leg.asset.symbol}
                  </p>
                  <p className="truncate text-muted-foreground text-xs">
                    {leg.counterparty
                      ? `${n > 0 ? t("transactions.detail.from") : t("transactions.detail.to")} ${shortAddress(leg.counterparty)}`
                      : leg.asset.name}
                  </p>
                </div>
                {leg.valueUsd !== null && (
                  <span className="tabular text-muted-foreground text-sm">
                    {format(Math.abs(Number(leg.valueUsd)))}
                  </span>
                )}
              </div>
            );
          })}
          {tx.legs.length === 0 && (
            <p className="text-muted-foreground text-sm">{t("transactions.detail.noMovement")}</p>
          )}
        </section>

        <dl className="divide-y divide-border rounded-xl border text-sm">
          <Row label={t("transactions.detail.wallet")}>{tx.walletLabel}</Row>
          <Row label={t("transactions.detail.network")}>
            <span className="flex items-center gap-1.5">
              <span
                className="size-2 rounded-full"
                style={{ background: CHAINS[tx.chain].color }}
              />
              {CHAINS[tx.chain].name}
            </span>
          </Row>
          {tx.fee && (
            <Row label={t("transactions.detail.fee")}>
              <span className="tabular">
                {amount.format(Number(tx.fee.amount))} {tx.fee.asset.symbol}
                {tx.fee.valueUsd !== null && (
                  <span className="text-muted-foreground">
                    {" "}
                    · {format(Number(tx.fee.valueUsd))}
                  </span>
                )}
              </span>
            </Row>
          )}
          <Row label={t("transactions.detail.hash")}>
            <button
              type="button"
              onClick={() => {
                void navigator.clipboard.writeText(tx.hash);
                setCopied(true);
              }}
              className="inline-flex items-center gap-1.5 font-mono text-xs hover:text-primary"
            >
              {shortAddress(tx.hash, 8, 6)}
              {copied ? <Check className="size-3.5 text-success" /> : <Copy className="size-3.5" />}
            </button>
          </Row>
        </dl>

        <Button
          variant="outline"
          className="w-full"
          onClick={() => openExternal(txUrl(tx.chain, tx.hash))}
        >
          <ExternalLink />
          {t("transactions.detail.explorer")}
        </Button>

        <section className="space-y-4 border-t pt-6">
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="tx-kind">{t("transactions.detail.type")}</Label>
              {kind !== tx.autoKind && (
                <button
                  type="button"
                  onClick={() => setKind(tx.autoKind)}
                  className="inline-flex items-center gap-1 text-muted-foreground text-xs hover:text-foreground"
                >
                  <RotateCcw className="size-3" />
                  {t("transactions.detail.reset", { kind: t(`transactions.kinds.${tx.autoKind}`) })}
                </button>
              )}
            </div>
            <Select value={kind} onValueChange={(v) => setKind(v as TxKind)}>
              <SelectTrigger id="tx-kind" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TX_KINDS.map((k) => (
                  <SelectItem key={k} value={k}>
                    {t(`transactions.kinds.${k}`)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-muted-foreground text-xs">{t("transactions.detail.typeHint")}</p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="tx-note">{t("transactions.detail.note")}</Label>
            <textarea
              id="tx-note"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              rows={3}
              placeholder={t("transactions.detail.notePlaceholder")}
              className="w-full resize-none rounded-lg border bg-card px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-primary"
            />
          </div>
          <Button className="w-full" disabled={!dirty || update.isPending} onClick={save}>
            {t("transactions.detail.save")}
          </Button>
        </section>
      </div>
    </>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 truncate text-right font-medium">{children}</dd>
    </div>
  );
}
