import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, KeyRound, Loader2, PlugZap } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Panel } from "@/components/panel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type ApiError, type ApiKeyStatus, type ApiProvider, api } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

const KEYS_KEY = ["apiKeys"] as const;

const PROVIDERS: Record<ApiProvider, { name: string; url: string }> = {
  etherscan: { name: "Etherscan V2", url: "etherscan.io/apis" },
  helius: { name: "Helius", url: "helius.dev" },
  coingecko: { name: "CoinGecko", url: "coingecko.com/en/api" },
  trongrid: { name: "TronGrid", url: "trongrid.io" },
};

/** Settings section: BYO API keys kept in the OS keychain. */
export function DataSourcesPanel() {
  const { t } = useTranslation();
  const { data } = useQuery({ queryKey: KEYS_KEY, queryFn: api.listApiKeys, enabled: IS_TAURI });

  return (
    <Panel title={t("dataSources.title")} subtitle={t("dataSources.subtitle")}>
      <ul className="divide-y divide-border">
        {(data ?? []).map((status) => (
          <ProviderRow key={status.provider} status={status} />
        ))}
      </ul>
      {!IS_TAURI && <p className="text-muted-foreground text-sm">{t("wallets.desktopOnly")}</p>}
    </Panel>
  );
}

function ProviderRow({ status }: { status: ApiKeyStatus }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [key, setKey] = useState("");
  const info = PROVIDERS[status.provider];
  const refresh = () => queryClient.invalidateQueries({ queryKey: KEYS_KEY });

  const test = useMutation<void, ApiError, string | undefined>({
    mutationFn: (candidate) => api.testApiKey(status.provider, candidate),
    onSuccess: () => toast.success(t("dataSources.testOk", { provider: info.name })),
    onError: (error) =>
      toast.error(t("dataSources.testFailed", { provider: info.name }), {
        description: t(`errors.${error.code}`),
      }),
  });
  const save = useMutation<void, ApiError, string>({
    mutationFn: (value) => api.setApiKey(status.provider, value),
    onSuccess: () => {
      toast.success(t("dataSources.saved", { provider: info.name }));
      setEditing(false);
      setKey("");
      refresh();
    },
    onError: (error) => toast.error(t(`errors.${error.code}`)),
  });
  const remove = useMutation<void, ApiError>({
    mutationFn: () => api.removeApiKey(status.provider),
    onSuccess: refresh,
    onError: (error) => toast.error(t(`errors.${error.code}`)),
  });

  const busy = test.isPending || save.isPending || remove.isPending;

  return (
    <li className="space-y-3 py-4 first:pt-0 last:pb-0">
      <div className="flex items-center justify-between gap-6">
        <div className="flex min-w-0 items-start gap-3">
          <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <PlugZap className="size-4" />
          </span>
          <div className="min-w-0 space-y-0.5">
            <p className="flex items-center gap-2 font-medium text-sm">
              {info.name}
              {status.configured && (
                <span className="inline-flex items-center gap-1 rounded-full bg-success/12 px-2 py-0.5 font-medium text-success text-xs">
                  <CheckCircle2 className="size-3" />
                  {t("dataSources.configured", { hint: status.hint })}
                </span>
              )}
            </p>
            <p className="text-muted-foreground text-sm">
              {t(`dataSources.hints.${status.provider}`)}{" "}
              <span className="font-mono text-xs">{info.url}</span>
            </p>
          </div>
        </div>
        {!editing && (
          <div className="flex shrink-0 gap-2">
            {status.configured && (
              <>
                <Button
                  variant="ghost"
                  disabled={busy}
                  onClick={() => test.mutate(undefined)}
                  className="rounded-lg"
                >
                  {test.isPending && <Loader2 className="animate-spin" />}
                  {t("dataSources.test")}
                </Button>
                <Button
                  variant="ghost"
                  disabled={busy}
                  onClick={() => remove.mutate()}
                  className="rounded-lg hover:text-danger"
                >
                  {t("security.remove")}
                </Button>
              </>
            )}
            <Button variant="outline" onClick={() => setEditing(true)} className="rounded-lg">
              <KeyRound />
              {status.configured ? t("dataSources.replace") : t("dataSources.add")}
            </Button>
          </div>
        )}
      </div>
      {editing && (
        <form
          className="flex gap-2 pl-12"
          onSubmit={(e) => {
            e.preventDefault();
            if (key.trim()) save.mutate(key.trim());
          }}
        >
          <Input
            type="password"
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder={t("dataSources.placeholder", { provider: info.name })}
            aria-label={t("dataSources.placeholder", { provider: info.name })}
            autoComplete="off"
            spellCheck={false}
            autoFocus
            className="h-9 rounded-lg font-mono"
          />
          <Button
            type="button"
            variant="ghost"
            disabled={!key.trim() || busy}
            onClick={() => test.mutate(key.trim())}
            className="rounded-lg"
          >
            {test.isPending && <Loader2 className="animate-spin" />}
            {t("dataSources.test")}
          </Button>
          <Button type="submit" disabled={!key.trim() || busy} className="rounded-lg">
            {t("wallets.save")}
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={() => {
              setEditing(false);
              setKey("");
            }}
            className="rounded-lg"
          >
            {t("wallets.cancel")}
          </Button>
        </form>
      )}
    </li>
  );
}
