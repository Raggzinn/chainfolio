import { Outlet } from "@tanstack/react-router";
import { useState } from "react";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { AppSidebar } from "./app-sidebar";
import { CommandMenu } from "./command-menu";
import { Topbar } from "./topbar";

export function AppLayout() {
  const [commandOpen, setCommandOpen] = useState(false);

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <Topbar onOpenCommand={() => setCommandOpen(true)} />
        <main className="mx-auto w-full max-w-7xl flex-1 space-y-6 p-6">
          <Outlet />
        </main>
      </SidebarInset>
      <CommandMenu open={commandOpen} onOpenChange={setCommandOpen} />
    </SidebarProvider>
  );
}
