import { CheckCircle2, ExternalLink, KeyRound } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Panel } from "@/components/panel";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useApiKeys, useSetApiKey, useSyncHistory } from "@/features/transactions/queries";
import { openExternal } from "@/lib/explorers";
import { IS_TAURI } from "@/lib/platform";

const ETHERSCAN_SIGNUP = "https://etherscan.io/myapikey";

/** Optional provider keys. Values are write-only: the UI only learns whether one is set. */
export function DataSourcesPanel() {
  const { t } = useTranslation();
  const keys = useApiKeys();
  const setKey = useSetApiKey();
  const sync = useSyncHistory();
  const [value, setValue] = useState("");
  const configured = keys.data?.etherscan ?? false;

  const save = (key: string | null) =>
    setKey.mutate(
      { provider: "etherscan", key },
      {
        onSuccess: () => {
          setValue("");
          toast.success(key ? t("dataSources.saved") : t("dataSources.removed"));
          if (key) sync.mutate(undefined);
        },
        onError: (e) => toast.error(e.message),
      },
    );

  return (
    <Panel title={t("dataSources.title")} subtitle={t("dataSources.subtitle")}>
      <div className="space-y-3">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-1">
            <Label htmlFor="etherscan-key" className="flex items-center gap-2 text-sm">
              <KeyRound className="size-4 text-muted-foreground" />
              Etherscan V2
              {configured && (
                <span className="inline-flex items-center gap-1 rounded-full bg-success/12 px-2 py-0.5 font-medium text-success text-xs">
                  <CheckCircle2 className="size-3" />
                  {t("dataSources.configured")}
                </span>
              )}
            </Label>
            <p className="max-w-lg text-muted-foreground text-sm">
              {t("dataSources.etherscanHint")}
            </p>
          </div>
          <Button variant="ghost" size="sm" onClick={() => openExternal(ETHERSCAN_SIGNUP)}>
            <ExternalLink />
            {t("dataSources.getKey")}
          </Button>
        </div>
        <div className="flex gap-2">
          <input
            id="etherscan-key"
            type="password"
            autoComplete="off"
            spellCheck={false}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            disabled={!IS_TAURI}
            placeholder={
              configured ? t("dataSources.replacePlaceholder") : t("dataSources.placeholder")
            }
            className="h-9 flex-1 rounded-lg border bg-card px-3 font-mono text-sm outline-none transition-colors placeholder:font-sans placeholder:text-muted-foreground focus-visible:border-primary"
          />
          <Button disabled={!value.trim() || setKey.isPending} onClick={() => save(value.trim())}>
            {t("dataSources.save")}
          </Button>
          {configured && (
            <Button variant="outline" disabled={setKey.isPending} onClick={() => save(null)}>
              {t("dataSources.remove")}
            </Button>
          )}
        </div>
        <p className="text-muted-foreground text-xs">{t("dataSources.privacy")}</p>
      </div>
    </Panel>
  );
}
