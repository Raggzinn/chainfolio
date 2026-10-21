import { Link } from "@tanstack/react-router";
import { ChevronRight } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Panel } from "@/components/panel";
import { Skeleton } from "@/components/ui/skeleton";
import type { TxItem } from "@/lib/api";
import { useIsSyncingHistory, useRecentTransactions } from "./queries";
import { TxDetailSheet } from "./tx-detail-sheet";
import { TxRow } from "./tx-row";

/** Latest transactions across every wallet, on the dashboard. */
export function RecentActivityCard() {
  const { t } = useTranslation();
  const { data, isPending } = useRecentTransactions(6);
  const syncing = useIsSyncingHistory();
  const [open, setOpen] = useState<TxItem | null>(null);

  return (
    <Panel
      title={t("dashboard.recentActivity")}
      action={
        <Link
          to="/transactions"
          className="group flex items-center gap-0.5 font-medium text-primary text-xs"
        >
          {t("dashboard.viewAll")}
          <ChevronRight className="size-3.5 transition-transform group-hover:translate-x-0.5" />
        </Link>
      }
      className="pb-2"
    >
      <div className="-mx-3">
        {isPending ? (
          <div className="space-y-2 px-3">
            {[0, 1, 2].map((i) => (
              <Skeleton key={i} className="h-12 rounded-xl" />
            ))}
          </div>
        ) : data && data.items.length > 0 ? (
          data.items.map((tx) => <TxRow key={`${tx.id}-${tx.walletId}`} tx={tx} onOpen={setOpen} />)
        ) : (
          <p className="px-3 py-6 text-center text-muted-foreground text-sm">
            {syncing ? t("transactions.syncing") : t("transactions.empty")}
          </p>
        )}
      </div>
      <TxDetailSheet tx={open} onClose={() => setOpen(null)} />
    </Panel>
  );
}
