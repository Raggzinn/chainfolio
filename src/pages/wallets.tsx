import { Plus, Wallet } from "lucide-react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";

export function WalletsPage() {
  const { t } = useTranslation();
  return (
    <>
      <PageHeader
        title={t("pages.wallets.title")}
        description={t("pages.wallets.description")}
        actions={
          <Button disabled>
            <Plus />
            {t("topbar.addWallet")}
          </Button>
        }
      />
      <EmptyState
        icon={Wallet}
        title={t("empty.noWallets")}
        description={t("empty.noWalletsHint")}
      />
    </>
  );
}
