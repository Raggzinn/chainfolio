import { Link, useRouterState } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand/brand-mark";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { usePreferences } from "@/stores/preferences";
import { visibleNavGroups } from "./nav-items";

export function AppSidebar() {
  const { t } = useTranslation();
  const taxModuleEnabled = usePreferences((s) => s.taxModuleEnabled);
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader>
        <Link to="/" className="flex items-center gap-2.5 px-1.5 py-1">
          <BrandMark className="size-7 shrink-0" />
          <span className="font-semibold text-lg tracking-tight group-data-[collapsible=icon]:hidden">
            Chain<span className="text-primary">folio</span>
          </span>
        </Link>
      </SidebarHeader>

      <SidebarContent>
        {visibleNavGroups(taxModuleEnabled).map((group) => (
          <SidebarGroup key={group.label}>
            <SidebarGroupLabel>{t(`nav.${group.label}`)}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {group.items.map((item) => {
                  const active = item.to === "/" ? pathname === "/" : pathname.startsWith(item.to);
                  return (
                    <SidebarMenuItem key={item.key}>
                      <SidebarMenuButton asChild isActive={active} tooltip={t(`nav.${item.key}`)}>
                        <Link to={item.to}>
                          <item.icon />
                          <span>{t(`nav.${item.key}`)}</span>
                        </Link>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  );
                })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        ))}
      </SidebarContent>

      <SidebarFooter>
        <p className="px-2 text-muted-foreground text-xs group-data-[collapsible=icon]:hidden">
          v{__APP_VERSION__}
        </p>
      </SidebarFooter>
    </Sidebar>
  );
}
