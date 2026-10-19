import { Link } from "@tanstack/react-router";
import { Plus, Wallet } from "lucide-react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { formatCurrency } from "@/lib/format";
import { usePreferences } from "@/stores/preferences";

export function PortfolioPage() {
  const { t, i18n } = useTranslation();
  const currency = usePreferences((s) => s.currency);
  const privacyMode = usePreferences((s) => s.privacyMode);

  return (
    <>
      <PageHeader
        title={t("pages.portfolio.title")}
        description={t("pages.portfolio.description")}
        actions={<AddWalletButton />}
      />

      <Card>
        <CardHeader>
          <CardDescription>{t("pages.portfolio.totalValue")}</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="tabular font-semibold text-4xl tracking-tight">
            {formatCurrency(0, currency, i18n.language, { hidden: privacyMode })}
          </p>
        </CardContent>
      </Card>

      <EmptyState
        icon={Wallet}
        title={t("empty.noWallets")}
        description={t("empty.noWalletsHint")}
        action={<AddWalletButton />}
      />
    </>
  );
}

function AddWalletButton() {
  const { t } = useTranslation();
  return (
    <Button asChild>
      <Link to="/wallets">
        <Plus />
        {t("topbar.addWallet")}
      </Link>
    </Button>
  );
}
