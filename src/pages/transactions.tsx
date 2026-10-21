import { Link } from "@tanstack/react-router";
import { ArrowLeftRight, Download, KeyRound, RefreshCw, Search, Wallet, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { DateRange } from "react-day-picker";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { EmptyState } from "@/components/layout/empty-state";
import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { DateRangePicker } from "@/features/portfolio/date-range-picker";
import {
  downloadTransactionsCsv,
  useApiKeys,
  useIsSyncingHistory,
  useSyncHistory,
  useTransactions,
} from "@/features/transactions/queries";
import { TxDetailSheet } from "@/features/transactions/tx-detail-sheet";
import { TxRow } from "@/features/transactions/tx-row";
import { usePortfolio } from "@/features/wallets/queries";
import { type ChainId, TX_KINDS, type TxItem, type TxKind, type TxQuery } from "@/lib/api";
import { CHAINS } from "@/lib/chains";
import { cn } from "@/lib/utils";

const ALL = "all";
/** EVM chains that only have history with an Etherscan key. */
const KEYED_CHAINS: ChainId[] = ["arbitrum", "base", "polygon", "bsc"];

export function TransactionsPage() {
  const { t, i18n } = useTranslation();
  const { data: portfolio } = usePortfolio();
  const wallets = portfolio?.wallets ?? [];
  const [search, setSearch] = useState("");
  const [debounced, setDebounced] = useState("");
  const [walletId, setWalletId] = useState<string>(ALL);
  const [chain, setChain] = useState<string>(ALL);
  const [kind, setKind] = useState<string>(ALL);
  const [assetId, setAssetId] = useState<string>(ALL);
  const [range, setRange] = useState<DateRange>();
  const [open, setOpen] = useState<TxItem | null>(null);

  useEffect(() => {
    const id = window.setTimeout(() => setDebounced(search), 250);
    return () => window.clearTimeout(id);
  }, [search]);

  const query: TxQuery = useMemo(
    () => ({
      search: debounced || null,
      walletId: walletId === ALL ? null : Number(walletId),
      chain: chain === ALL ? null : (chain as ChainId),
      kind: kind === ALL ? null : (kind as TxKind),
      assetId: assetId === ALL ? null : assetId,
      from: range?.from ? startOfDay(range.from) : null,
      to: range?.to ? endOfDay(range.to) : null,
    }),
    [debounced, walletId, chain, kind, assetId, range],
  );
  const filtered = Object.values(query).some((v) => v !== null);

  const list = useTransactions(query);
  const items = useMemo(() => list.data?.pages.flatMap((p) => p.items) ?? [], [list.data]);
  const total = list.data?.pages[0]?.total ?? 0;
  const groups = useMemo(() => groupByDay(items), [items]);
  const syncing = useIsSyncingHistory();
  const sync = useSyncHistory();
  const keys = useApiKeys();
  const needsKey = wallets.some((w) => w.family === "evm") && keys.data?.etherscan === false;

  const sentinel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = sentinel.current;
    if (!el) return;
    const observer = new IntersectionObserver(([entry]) => {
      if (entry?.isIntersecting && list.hasNextPage && !list.isFetchingNextPage)
        list.fetchNextPage();
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [list]);

  const chains = useMemo(() => [...new Set(wallets.flatMap((w) => w.chains))].sort(), [wallets]);
  const assets = portfolio?.assets ?? [];
  const day = new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "full" });

  const reset = () => {
    setSearch("");
    setWalletId(ALL);
    setChain(ALL);
    setKind(ALL);
    setAssetId(ALL);
    setRange(undefined);
  };

  if (wallets.length === 0) {
    return (
      <>
        <PageHeader
          title={t("pages.transactions.title")}
          description={t("pages.transactions.description")}
        />
        <EmptyState
          icon={Wallet}
          title={t("empty.noWallets")}
          description={t("transactions.noWalletsHint")}
          action={
            <Button asChild>
              <Link to="/wallets">{t("topbar.addWallet")}</Link>
            </Button>
          }
        />
      </>
    );
  }

  return (
    <>
      <PageHeader
        title={t("pages.transactions.title")}
        description={t("pages.transactions.description")}
        actions={
          <>
            <Button variant="outline" disabled={syncing} onClick={() => sync.mutate(undefined)}>
              <RefreshCw className={cn(syncing && "animate-spin")} />
              {syncing ? t("transactions.syncing") : t("transactions.sync")}
            </Button>
            <Button
              variant="outline"
              disabled={total === 0}
              onClick={() =>
                downloadTransactionsCsv(query)
                  .then(() => toast.success(t("transactions.exported")))
                  .catch((e: Error) => toast.error(e.message))
              }
            >
              <Download />
              {t("transactions.export")}
            </Button>
          </>
        }
      />

      {needsKey && (
        <div className="surface flex items-center gap-3 rounded-xl px-4 py-3 text-sm">
          <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-warning/12 text-warning">
            <KeyRound className="size-4" />
          </span>
          <p className="flex-1 text-muted-foreground">
            {t("transactions.needsKey", {
              chains: KEYED_CHAINS.map((c) => CHAINS[c].name).join(", "),
            })}
          </p>
          <Button asChild size="sm" variant="outline">
            <Link to="/settings">{t("transactions.addKey")}</Link>
          </Button>
        </div>
      )}

      <div className="surface flex flex-wrap items-center gap-2 rounded-2xl p-3">
        <div className="relative min-w-56 flex-1">
          <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t("transactions.searchPlaceholder")}
            className="h-9 w-full rounded-xl border bg-card pr-3 pl-9 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-primary"
          />
        </div>
        <Filter value={walletId} onChange={setWalletId} all={t("transactions.allWallets")}>
          {wallets.map((w) => (
            <SelectItem key={w.id} value={String(w.id)}>
              {w.label}
            </SelectItem>
          ))}
        </Filter>
        <Filter value={chain} onChange={setChain} all={t("transactions.allChains")}>
          {chains.map((c) => (
            <SelectItem key={c} value={c}>
              {CHAINS[c].name}
            </SelectItem>
          ))}
        </Filter>
        <Filter value={kind} onChange={setKind} all={t("transactions.allTypes")}>
          {TX_KINDS.map((k) => (
            <SelectItem key={k} value={k}>
              {t(`transactions.kinds.${k}`)}
            </SelectItem>
          ))}
        </Filter>
        <Filter value={assetId} onChange={setAssetId} all={t("filters.allAssets")}>
          {assets.map((a) => (
            <SelectItem key={a.id} value={a.id}>
              {a.symbol} · {CHAINS[a.chain].name}
            </SelectItem>
          ))}
        </Filter>
        <DateRangePicker value={range} active={Boolean(range?.from)} onChange={setRange} />
        {filtered && (
          <Button variant="ghost" size="sm" onClick={reset}>
            <X />
            {t("transactions.clear")}
          </Button>
        )}
      </div>

      <section className="surface rounded-2xl p-2">
        {list.isPending ? (
          <ListSkeleton />
        ) : items.length === 0 ? (
          <EmptyState
            icon={ArrowLeftRight}
            title={filtered ? t("transactions.emptyFiltered") : t("transactions.empty")}
            description={filtered ? undefined : t("transactions.emptyHint")}
          />
        ) : (
          <>
            <p className="px-3 pt-2 pb-1 text-muted-foreground text-xs">
              {t("transactions.count", { count: total })}
            </p>
            {groups.map(([key, txs]) => (
              <div key={key}>
                <h3 className="sticky top-0 z-10 bg-card px-3 pt-4 pb-1.5 font-medium text-muted-foreground text-xs capitalize">
                  {day.format(txs[0]?.timestamp ?? 0)}
                </h3>
                {txs.map((tx) => (
                  <TxRow key={`${tx.id}-${tx.walletId}`} tx={tx} onOpen={setOpen} />
                ))}
              </div>
            ))}
            <div ref={sentinel} className="h-8" />
            {list.isFetchingNextPage && <ListSkeleton rows={2} />}
          </>
        )}
      </section>

      <TxDetailSheet tx={open} onClose={() => setOpen(null)} />
    </>
  );
}

function Filter({
  value,
  onChange,
  all,
  children,
}: {
  value: string;
  onChange: (v: string) => void;
  all: string;
  children: React.ReactNode;
}) {
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger
        size="sm"
        className={cn(
          "h-9 w-auto min-w-36 rounded-xl",
          value !== ALL && "border-primary/40 text-primary",
        )}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={ALL}>{all}</SelectItem>
        {children}
      </SelectContent>
    </Select>
  );
}

function ListSkeleton({ rows = 6 }: { rows?: number }) {
  return (
    <div className="space-y-2 p-2">
      {Array.from({ length: rows }, (_, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: static placeholders
        <Skeleton key={i} className="h-14 rounded-xl" />
      ))}
    </div>
  );
}

function startOfDay(d: Date) {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

function endOfDay(d: Date) {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate(), 23, 59, 59, 999).getTime();
}

/** Groups transactions (already newest first) by local calendar day. */
export function groupByDay(items: TxItem[]): [string, TxItem[]][] {
  const groups = new Map<string, TxItem[]>();
  for (const tx of items) {
    const d = new Date(tx.timestamp);
    const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
    const group = groups.get(key);
    if (group) group.push(tx);
    else groups.set(key, [tx]);
  }
  return [...groups];
}
