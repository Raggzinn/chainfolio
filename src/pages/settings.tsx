import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { useTheme } from "@/hooks/use-theme";
import { SUPPORTED_LANGUAGES } from "@/i18n";
import { CURRENCIES, type Currency, type Theme, usePreferences } from "@/stores/preferences";

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const { currency, setCurrency, taxModuleEnabled, setTaxModuleEnabled } = usePreferences();

  return (
    <>
      <PageHeader title={t("pages.settings.title")} description={t("pages.settings.description")} />

      <Card>
        <CardHeader>
          <CardTitle>{t("settings.general")}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-5">
          <SettingRow
            id="language"
            label={t("settings.language")}
            hint={t("settings.languageHint")}
          >
            <Select value={i18n.resolvedLanguage} onValueChange={(v) => i18n.changeLanguage(v)}>
              <SelectTrigger id="language" className="w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {SUPPORTED_LANGUAGES.map((l) => (
                  <SelectItem key={l.code} value={l.code}>
                    {l.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </SettingRow>
          <Separator />
          <SettingRow id="theme" label={t("settings.theme")} hint={t("settings.themeHint")}>
            <Select value={theme} onValueChange={(v) => setTheme(v as Theme)}>
              <SelectTrigger id="theme" className="w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="dark">{t("settings.themeDark")}</SelectItem>
                <SelectItem value="light">{t("settings.themeLight")}</SelectItem>
                <SelectItem value="system">{t("settings.themeSystem")}</SelectItem>
              </SelectContent>
            </Select>
          </SettingRow>
          <Separator />
          <SettingRow
            id="currency"
            label={t("settings.currency")}
            hint={t("settings.currencyHint")}
          >
            <Select value={currency} onValueChange={(v) => setCurrency(v as Currency)}>
              <SelectTrigger id="currency" className="w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CURRENCIES.map((c) => (
                  <SelectItem key={c} value={c}>
                    {c}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </SettingRow>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("settings.modules")}</CardTitle>
        </CardHeader>
        <CardContent>
          <SettingRow
            id="tax-module"
            label={t("settings.taxModule")}
            hint={t("settings.taxModuleHint")}
          >
            <Switch
              id="tax-module"
              checked={taxModuleEnabled}
              onCheckedChange={setTaxModuleEnabled}
            />
          </SettingRow>
        </CardContent>
      </Card>
    </>
  );
}

function SettingRow({
  id,
  label,
  hint,
  children,
}: {
  id: string;
  label: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6">
      <div className="space-y-1">
        <Label htmlFor={id}>{label}</Label>
        <p className="text-muted-foreground text-sm">{hint}</p>
      </div>
      {children}
    </div>
  );
}
