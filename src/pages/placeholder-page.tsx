import { Construction } from "lucide-react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";

type PageKey = "assets" | "transactions" | "analytics" | "defi" | "risk" | "taxes" | "alerts";

export function PlaceholderPage({ page }: { page: PageKey }) {
  const { t } = useTranslation();
  return (
    <>
      <PageHeader title={t(`pages.${page}.title`)} description={t(`pages.${page}.description`)} />
      <EmptyState icon={Construction} title={t("empty.comingSoon")} />
    </>
  );
}
