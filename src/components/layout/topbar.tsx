import { useNavigate } from "@tanstack/react-router";
import { CircleHelp, Command, Eye, EyeOff, Moon, RefreshCw, Search, Sun } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useHasWallets, useIsSyncing, useRefreshBalances } from "@/features/wallets/queries";
import { useTheme } from "@/hooks/use-theme";
import { gsap } from "@/lib/gsap";
import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { usePreferences } from "@/stores/preferences";
import { useTour } from "@/stores/tour";
import { WindowControls } from "./window-controls";

export function Topbar({ onOpenCommand }: { onOpenCommand: () => void }) {
  const { t } = useTranslation();
  const { resolvedTheme, toggleTheme } = useTheme();
  const privacyMode = usePreferences((s) => s.privacyMode);
  const togglePrivacyMode = usePreferences((s) => s.togglePrivacyMode);
  const demoMode = usePreferences((s) => s.demoMode);
  const hasWallets = useHasWallets();
  const refresh = useRefreshBalances();
  const syncing = useIsSyncing();
  const syncIcon = useRef<SVGSVGElement>(null);
  const themeIcon = useRef<HTMLSpanElement>(null);
  const startTour = useTour((s) => s.start);
  const navigate = useNavigate();

  const spinSync = () => {
    gsap.fromTo(
      syncIcon.current,
      { rotate: 0 },
      { rotate: 360, duration: 0.9, ease: "power2.inOut" },
    );
    if (hasWallets && !demoMode && !syncing) refresh.mutate(undefined);
  };

  const flipTheme = () => {
    gsap.fromTo(
      themeIcon.current,
      { rotate: -90, scale: 0.4, autoAlpha: 0 },
      { rotate: 0, scale: 1, autoAlpha: 1, duration: 0.5, ease: "back.out(2)" },
    );
    toggleTheme();
  };

  return (
    <header data-tauri-drag-region className="flex h-[60px] shrink-0 items-center gap-3 pr-4 pl-5">
      <button
        type="button"
        data-tour="search"
        onClick={onOpenCommand}
        className="surface group flex h-10 w-full max-w-md items-center gap-2.5 rounded-xl px-3.5 text-muted-foreground text-sm transition-all duration-300 hover:text-foreground hover:ring-1 hover:ring-primary/30"
      >
        <Search className="size-4 transition-transform duration-300 group-hover:scale-110" />
        <span className="flex-1 text-left">{t("topbar.search")}</span>
        <kbd className="flex items-center gap-0.5 rounded-md border bg-background/60 px-1.5 py-0.5 font-mono text-[10px]">
          {IS_MAC ? <Command className="size-3" /> : "Ctrl"} K
        </kbd>
      </button>

      <div data-tauri-drag-region className="h-full flex-1" />

      <div className="surface flex items-center gap-0.5 rounded-xl p-1">
        <IconButton label={t("topbar.sync")} onClick={spinSync}>
          <RefreshCw ref={syncIcon} className={cn(syncing && "animate-spin")} />
        </IconButton>
        <IconButton
          tourId="privacy"
          label={privacyMode ? t("topbar.privacyOff") : t("topbar.privacyOn")}
          onClick={togglePrivacyMode}
          active={privacyMode}
        >
          {privacyMode ? <EyeOff /> : <Eye />}
        </IconButton>
        <IconButton label={t("command.toggleTheme")} onClick={flipTheme}>
          <span ref={themeIcon} className="flex">
            {resolvedTheme === "dark" ? <Sun /> : <Moon />}
          </span>
        </IconButton>
        <IconButton
          label={t("tour.start")}
          onClick={() => {
            navigate({ to: "/" });
            startTour();
          }}
        >
          <CircleHelp />
        </IconButton>
      </div>

      <WindowControls />
    </header>
  );
}

function IconButton({
  label,
  onClick,
  active,
  tourId,
  children,
}: {
  label: string;
  tourId?: string;
  onClick: () => void;
  active?: boolean;
  children: React.ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          data-tour={tourId}
          onClick={onClick}
          aria-label={label}
          aria-pressed={active}
          className={cn(
            "flex size-8 items-center justify-center rounded-lg text-muted-foreground transition-all duration-200 hover:bg-accent hover:text-foreground active:scale-90 [&_svg]:size-4",
            active && "bg-primary/15 text-primary",
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
