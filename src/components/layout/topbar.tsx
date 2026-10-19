import { Eye, EyeOff, Moon, RefreshCw, Search, Sun } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useTheme } from "@/hooks/use-theme";
import { usePreferences } from "@/stores/preferences";

export function Topbar({ onOpenCommand }: { onOpenCommand: () => void }) {
  const { t } = useTranslation();
  const { resolvedTheme, toggleTheme } = useTheme();
  const privacyMode = usePreferences((s) => s.privacyMode);
  const togglePrivacyMode = usePreferences((s) => s.togglePrivacyMode);

  return (
    <header className="sticky top-0 z-10 flex h-14 shrink-0 items-center gap-2 border-b bg-background/80 px-4 backdrop-blur">
      <SidebarTrigger className="-ml-1" />
      <Separator orientation="vertical" className="mr-1 data-[orientation=vertical]:h-4" />

      <button
        type="button"
        onClick={onOpenCommand}
        className="flex h-9 w-full max-w-sm items-center gap-2 rounded-md border bg-muted/40 px-3 text-muted-foreground text-sm transition-colors hover:bg-muted"
      >
        <Search className="size-4" />
        <span className="flex-1 text-left">{t("topbar.search")}</span>
        <kbd className="rounded border bg-background px-1.5 font-mono text-[10px]">Ctrl K</kbd>
      </button>

      <div className="ml-auto flex items-center gap-1">
        <IconButton label={t("topbar.sync")} onClick={() => {}}>
          <RefreshCw />
        </IconButton>
        <IconButton
          label={privacyMode ? t("topbar.privacyOff") : t("topbar.privacyOn")}
          onClick={togglePrivacyMode}
        >
          {privacyMode ? <EyeOff /> : <Eye />}
        </IconButton>
        <IconButton label={t("command.toggleTheme")} onClick={toggleTheme}>
          {resolvedTheme === "dark" ? <Sun /> : <Moon />}
        </IconButton>
      </div>
    </header>
  );
}

function IconButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="icon" onClick={onClick} aria-label={label}>
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
