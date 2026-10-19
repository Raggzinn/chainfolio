import { useNavigate } from "@tanstack/react-router";
import { Eye, SunMoon } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command";
import { useTheme } from "@/hooks/use-theme";
import { usePreferences } from "@/stores/preferences";
import { visibleNavGroups } from "./nav-items";

export function CommandMenu({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { toggleTheme } = useTheme();
  const taxModuleEnabled = usePreferences((s) => s.taxModuleEnabled);
  const togglePrivacyMode = usePreferences((s) => s.togglePrivacyMode);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        onOpenChange(!open);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, onOpenChange]);

  const run = (action: () => void) => {
    onOpenChange(false);
    action();
  };

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange}>
      <CommandInput placeholder={t("command.placeholder")} />
      <CommandList>
        <CommandEmpty>{t("command.empty")}</CommandEmpty>
        <CommandGroup heading={t("command.navigation")}>
          {visibleNavGroups(taxModuleEnabled)
            .flatMap((g) => g.items)
            .map((item) => (
              <CommandItem key={item.key} onSelect={() => run(() => navigate({ to: item.to }))}>
                <item.icon />
                {t(`nav.${item.key}`)}
              </CommandItem>
            ))}
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading={t("command.actions")}>
          <CommandItem onSelect={() => run(toggleTheme)}>
            <SunMoon />
            {t("command.toggleTheme")}
          </CommandItem>
          <CommandItem onSelect={() => run(togglePrivacyMode)}>
            <Eye />
            {t("command.togglePrivacy")}
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
