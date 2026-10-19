import { Link, useRouterState } from "@tanstack/react-router";
import { PanelLeftClose, PanelLeftOpen, ShieldCheck } from "lucide-react";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand/brand-mark";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { gsap, useGSAP } from "@/lib/gsap";
import { IS_MAC, IS_TAURI } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { usePreferences } from "@/stores/preferences";
import { visibleNavGroups } from "./nav-items";

const EXPANDED = 248;
const COLLAPSED = 76;

export function AppSidebar() {
  const { t } = useTranslation();
  const taxModuleEnabled = usePreferences((s) => s.taxModuleEnabled);
  const collapsed = usePreferences((s) => s.sidebarCollapsed);
  const toggleSidebar = usePreferences((s) => s.toggleSidebar);
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  const asideRef = useRef<HTMLElement>(null);
  const navRef = useRef<HTMLElement>(null);
  const indicatorRef = useRef<HTMLDivElement>(null);
  const firstRun = useRef(true);

  const groups = visibleNavGroups(taxModuleEnabled);
  const activeKey = groups
    .flatMap((g) => g.items)
    .find((item) => (item.to === "/" ? pathname === "/" : pathname.startsWith(item.to)))?.key;

  // Width + label fade when collapsing.
  useGSAP(
    () => {
      const instant = firstRun.current;
      gsap.to(asideRef.current, {
        width: collapsed ? COLLAPSED : EXPANDED,
        duration: instant ? 0 : 0.5,
        ease: "expo.inOut",
      });
      gsap.to("[data-label]", {
        autoAlpha: collapsed ? 0 : 1,
        x: collapsed ? -6 : 0,
        duration: instant ? 0 : 0.25,
        delay: collapsed || instant ? 0 : 0.2,
        stagger: collapsed ? 0 : 0.015,
      });
    },
    { scope: asideRef, dependencies: [collapsed] },
  );

  // Glide the active pill to the selected item.
  useGSAP(
    () => {
      const nav = navRef.current;
      const indicator = indicatorRef.current;
      const target = nav?.querySelector<HTMLElement>(`[data-nav="${activeKey}"]`);
      if (!nav || !indicator) return;
      if (!target) {
        gsap.to(indicator, { autoAlpha: 0, duration: 0.2 });
        return;
      }
      const navBox = nav.getBoundingClientRect();
      const box = target.getBoundingClientRect();
      gsap.to(indicator, {
        y: box.top - navBox.top + nav.scrollTop,
        height: box.height,
        autoAlpha: 1,
        duration: firstRun.current ? 0 : 0.55,
        ease: "expo.out",
      });
      firstRun.current = false;
    },
    { dependencies: [activeKey, taxModuleEnabled, collapsed] },
  );

  return (
    <aside
      ref={asideRef}
      style={{ width: collapsed ? COLLAPSED : EXPANDED }}
      className="relative z-20 flex h-full shrink-0 flex-col p-2.5 pr-0"
    >
      <div className="surface flex h-full flex-col overflow-hidden rounded-2xl">
        <div
          data-tauri-drag-region
          className={cn(
            "flex h-14 shrink-0 items-center gap-2.5 px-4",
            IS_TAURI && IS_MAC && "pt-6",
          )}
        >
          <Link to="/" className="flex items-center gap-2.5">
            <BrandMark className="size-8 shrink-0" />
            <span data-label className="whitespace-nowrap font-semibold text-[17px] tracking-tight">
              Chain<span className="text-primary">folio</span>
            </span>
          </Link>
        </div>

        <nav
          ref={navRef}
          data-tour="navigation"
          className="relative flex-1 space-y-5 overflow-y-auto overflow-x-hidden px-3 py-3"
        >
          <div
            ref={indicatorRef}
            aria-hidden="true"
            className="pointer-events-none invisible absolute top-0 right-3 left-3 rounded-xl bg-primary/12"
          >
            <span className="absolute top-1/2 left-0 h-5 w-[3px] -translate-y-1/2 rounded-full bg-primary" />
          </div>

          {groups.map((group) => (
            <div key={group.label} className="space-y-1">
              <p
                data-label
                className="px-3 pb-1 font-medium text-[11px] text-muted-foreground/80 uppercase tracking-[0.08em]"
              >
                {t(`nav.${group.label}`)}
              </p>
              {group.items.map((item) => {
                const active = item.key === activeKey;
                const link = (
                  <Link
                    key={item.key}
                    to={item.to}
                    data-nav={item.key}
                    data-tour={
                      item.key === "wallets" || item.key === "settings" ? item.key : undefined
                    }
                    className={cn(
                      "group relative flex h-10 items-center gap-3 rounded-xl px-3 font-medium text-sm transition-colors duration-200",
                      active ? "text-foreground" : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    <span className="absolute inset-0 rounded-xl bg-foreground/0 transition-colors group-hover:bg-accent" />
                    <item.icon
                      className={cn(
                        "relative size-[18px] shrink-0 transition-all duration-300 group-hover:scale-110",
                        active && "text-primary",
                      )}
                      strokeWidth={active ? 2.2 : 1.8}
                    />
                    <span data-label className="relative whitespace-nowrap">
                      {t(`nav.${item.key}`)}
                    </span>
                  </Link>
                );
                return collapsed ? (
                  <Tooltip key={item.key}>
                    <TooltipTrigger asChild>{link}</TooltipTrigger>
                    <TooltipContent side="right">{t(`nav.${item.key}`)}</TooltipContent>
                  </Tooltip>
                ) : (
                  <div key={item.key}>{link}</div>
                );
              })}
            </div>
          ))}
        </nav>

        <div className="space-y-2 p-3">
          <div data-label className="overflow-hidden rounded-xl bg-muted p-3">
            <div className="flex items-center gap-2 font-medium text-xs">
              <ShieldCheck className="size-4 text-success" />
              <span className="whitespace-nowrap">{t("sidebar.watchOnly")}</span>
              <span className="text-muted-foreground">·</span>
              <span className="whitespace-nowrap text-muted-foreground">{t("sidebar.local")}</span>
            </div>
            <div className="mt-2 flex items-center gap-2 text-[11px] text-muted-foreground">
              <span className="relative flex size-2">
                <span className="absolute inline-flex size-full animate-ping rounded-full bg-success opacity-60" />
                <span className="relative inline-flex size-2 rounded-full bg-success" />
              </span>
              <span className="whitespace-nowrap">{t("sidebar.synced")}</span>
              <span className="ml-auto font-mono">v{__APP_VERSION__}</span>
            </div>
          </div>

          <button
            type="button"
            onClick={toggleSidebar}
            aria-label={collapsed ? t("sidebar.expand") : t("sidebar.collapse")}
            className="flex h-9 w-full items-center gap-3 rounded-xl px-3 text-muted-foreground text-sm transition-colors hover:bg-accent hover:text-foreground"
          >
            {collapsed ? (
              <PanelLeftOpen className="size-[18px] shrink-0" />
            ) : (
              <PanelLeftClose className="size-[18px] shrink-0" />
            )}
            <span data-label className="whitespace-nowrap">
              {t("sidebar.collapse")}
            </span>
          </button>
        </div>
      </div>
    </aside>
  );
}
