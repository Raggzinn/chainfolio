import { Plus, RefreshCw, Wallet } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { AddWalletDialog } from "@/features/wallets/add-wallet-dialog";
import { useIsSyncing, usePortfolio, useRefreshBalances } from "@/features/wallets/queries";
import { WalletList } from "@/features/wallets/wallet-list";
import { IS_TAURI } from "@/lib/platform";
import { cn } from "@/lib/utils";

export function WalletsPage() {
  const { t } = useTranslation();
  const [adding, setAdding] = useState(false);
  const { data, isPending } = usePortfolio();
  const refresh = useRefreshBalances();
  const syncing = useIsSyncing();
  const wallets = data?.wallets ?? [];

  const addButton = (
    <Button onClick={() => setAdding(true)} disabled={!IS_TAURI} className="rounded-xl">
      <Plus />
      {t("topbar.addWallet")}
    </Button>
  );

  return (
    <>
      <PageHeader
        title={t("pages.wallets.title")}
        description={t("pages.wallets.description")}
        actions={
          <>
            {wallets.length > 0 && (
              <Button
                variant="outline"
                onClick={() => refresh.mutate(undefined)}
                disabled={syncing}
                className="rounded-xl"
              >
                <RefreshCw className={cn(syncing && "animate-spin")} />
                {syncing ? t("wallets.syncing") : t("topbar.sync")}
              </Button>
            )}
            {addButton}
          </>
        }
      />
      {IS_TAURI && isPending ? (
        <Skeleton className="h-40 rounded-2xl" />
      ) : wallets.length > 0 ? (
        <WalletList wallets={wallets} />
      ) : (
        <EmptyState
          icon={Wallet}
          title={t("empty.noWallets")}
          description={IS_TAURI ? t("empty.noWalletsHint") : t("wallets.desktopOnly")}
          action={addButton}
        />
      )}
      <AddWalletDialog open={adding} onOpenChange={setAdding} />
    </>
  );
}
