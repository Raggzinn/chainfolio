import { Check, Copy, Loader2, MoreHorizontal, Pencil, RefreshCw, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { NetworkIcon, TokenIcon } from "@/components/token-icon";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useMoney } from "@/hooks/use-money";
import { toApiError, type WalletSummary } from "@/lib/api";
import { CHAINS, shortAddress } from "@/lib/chains";
import { formatRelative } from "@/lib/format";
import { useIsSyncing, useRefreshBalances, useRemoveWallet, useRenameWallet } from "./queries";

const FAMILY_SYMBOL = { evm: "ETH", solana: "SOL", bitcoin: "BTC" } as const;

export function WalletList({ wallets }: { wallets: WalletSummary[] }) {
  const [renaming, setRenaming] = useState<WalletSummary | null>(null);
  const [removing, setRemoving] = useState<WalletSummary | null>(null);

  return (
    <>
      <ul className="surface divide-y divide-border overflow-hidden rounded-2xl">
        {wallets.map((wallet) => (
          <WalletRow
            key={wallet.id}
            wallet={wallet}
            onRename={() => setRenaming(wallet)}
            onRemove={() => setRemoving(wallet)}
          />
        ))}
      </ul>
      <RenameDialog wallet={renaming} onClose={() => setRenaming(null)} />
      <RemoveDialog wallet={removing} onClose={() => setRemoving(null)} />
    </>
  );
}

function WalletRow({
  wallet,
  onRename,
  onRemove,
}: {
  wallet: WalletSummary;
  onRename: () => void;
  onRemove: () => void;
}) {
  const { t, i18n } = useTranslation();
  const { format } = useMoney();
  const refresh = useRefreshBalances();
  const syncing = useIsSyncing();
  const [copied, setCopied] = useState(false);
  const shownAddress = wallet.input !== wallet.address ? wallet.input : wallet.address;

  const copy = async () => {
    await navigator.clipboard.writeText(wallet.address);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <li className="flex items-center gap-4 px-5 py-4 transition-colors hover:bg-accent/50">
      <TokenIcon symbol={FAMILY_SYMBOL[wallet.family]} size={40} />

      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex items-center gap-2">
          <p className="truncate font-semibold">{wallet.label}</p>
          {wallet.kind === "xpub" && (
            <span className="rounded-md bg-muted px-1.5 py-0.5 font-medium text-[10px] text-muted-foreground uppercase tracking-wider">
              xpub
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5 text-muted-foreground text-xs">
          <span className="truncate font-mono">{shortAddress(shownAddress, 10, 6)}</span>
          {wallet.input !== wallet.address && (
            <span className="truncate font-mono opacity-70">→ {shortAddress(wallet.address)}</span>
          )}
          <button
            type="button"
            onClick={copy}
            aria-label={t("wallets.copyAddress")}
            className="rounded p-0.5 transition-colors hover:text-foreground"
          >
            {copied ? <Check className="size-3.5 text-success" /> : <Copy className="size-3.5" />}
          </button>
        </div>
      </div>

      <div className="hidden items-center sm:flex">
        <span className="-space-x-1.5 flex">
          {wallet.chains.slice(0, 5).map((chain) => (
            <Tooltip key={chain}>
              <TooltipTrigger asChild>
                <span className="flex rounded-full ring-2 ring-card">
                  <NetworkIcon chain={chain} size={22} />
                </span>
              </TooltipTrigger>
              <TooltipContent>{CHAINS[chain].name}</TooltipContent>
            </Tooltip>
          ))}
        </span>
        {wallet.chains.length > 5 && (
          <span className="ml-1.5 text-muted-foreground text-xs">+{wallet.chains.length - 5}</span>
        )}
      </div>

      <div className="w-36 text-right">
        <p className="tabular font-semibold">{format(Number(wallet.totalUsd))}</p>
        <p className="text-muted-foreground text-xs">
          {wallet.syncedAt
            ? formatRelative(wallet.syncedAt, i18n.resolvedLanguage ?? "en")
            : t("wallets.neverSynced")}
        </p>
      </div>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon-sm" aria-label={t("wallets.actions")}>
            {refresh.isPending ? <Loader2 className="animate-spin" /> : <MoreHorizontal />}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-44">
          <DropdownMenuItem disabled={syncing} onSelect={() => refresh.mutate(wallet.id)}>
            <RefreshCw />
            {t("wallets.refresh")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onRename}>
            <Pencil />
            {t("wallets.rename")}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" onSelect={onRemove}>
            <Trash2 />
            {t("wallets.remove")}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  );
}

function RenameDialog({ wallet, onClose }: { wallet: WalletSummary | null; onClose: () => void }) {
  const { t } = useTranslation();
  const rename = useRenameWallet();
  const [label, setLabel] = useState("");

  return (
    <Dialog
      open={wallet !== null}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
          rename.reset();
        }
      }}
    >
      <DialogContent
        className="surface-raised rounded-2xl border-0 sm:max-w-sm"
        onOpenAutoFocus={() => wallet && setLabel(wallet.label)}
      >
        <DialogHeader>
          <DialogTitle>{t("wallets.renameTitle")}</DialogTitle>
          <DialogDescription>{wallet && shortAddress(wallet.address)}</DialogDescription>
        </DialogHeader>
        <form
          id="rename-wallet"
          onSubmit={(e) => {
            e.preventDefault();
            if (wallet) rename.mutate({ id: wallet.id, label }, { onSuccess: onClose });
          }}
        >
          <Input
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            maxLength={60}
            aria-label={t("wallets.nameLabel")}
            aria-invalid={Boolean(rename.error)}
            className="h-11 rounded-xl"
          />
          {rename.error && (
            <p role="alert" className="mt-2 text-danger text-xs">
              {t(`errors.${toApiError(rename.error).code}`)}
            </p>
          )}
        </form>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {t("wallets.cancel")}
          </Button>
          <Button type="submit" form="rename-wallet" disabled={rename.isPending}>
            {t("wallets.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RemoveDialog({ wallet, onClose }: { wallet: WalletSummary | null; onClose: () => void }) {
  const { t } = useTranslation();
  const remove = useRemoveWallet();

  return (
    <Dialog open={wallet !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="surface-raised rounded-2xl border-0 sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{t("wallets.removeTitle", { label: wallet?.label })}</DialogTitle>
          <DialogDescription>{t("wallets.removeDescription")}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {t("wallets.cancel")}
          </Button>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() =>
              wallet &&
              remove.mutate(wallet.id, {
                onSuccess: () => {
                  toast.success(t("wallets.removed", { label: wallet.label }));
                  onClose();
                },
                onError: (error) => toast.error(toApiError(error).message),
              })
            }
          >
            {t("wallets.remove")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
