import { Outlet, useRouterState } from "@tanstack/react-router";
import { useRef, useState } from "react";
import { ProductTour } from "@/features/onboarding/product-tour";
import { WelcomeScreen } from "@/features/onboarding/welcome-screen";
import { useAutoRefresh } from "@/features/wallets/queries";
import { gsap, MOTION_OK, useGSAP } from "@/lib/gsap";
import { AppSidebar } from "./app-sidebar";
import { CommandMenu } from "./command-menu";
import { Topbar } from "./topbar";

export function AppLayout() {
  const [commandOpen, setCommandOpen] = useState(false);
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const pageRef = useRef<HTMLDivElement>(null);
  useAutoRefresh();

  // Every page enters with a soft staggered rise of its direct children.
  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add(MOTION_OK, () => {
        gsap.from(pageRef.current?.children ?? [], {
          y: 18,
          autoAlpha: 0,
          filter: "blur(6px)",
          duration: 0.7,
          stagger: 0.06,
          ease: "expo.out",
          clearProps: "filter,transform",
        });
      });
      return () => mm.revert();
    },
    { dependencies: [pathname], scope: pageRef },
  );

  return (
    <div className="flex h-full">
      <AppSidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar onOpenCommand={() => setCommandOpen(true)} />
        <main className="flex-1 overflow-x-hidden overflow-y-auto px-5 pb-8">
          <div
            ref={pageRef}
            key={pathname}
            className="mx-auto w-full max-w-[1320px] space-y-5 pt-1"
          >
            <Outlet />
          </div>
        </main>
      </div>
      <CommandMenu open={commandOpen} onOpenChange={setCommandOpen} />
      <WelcomeScreen />
      <ProductTour />
    </div>
  );
}
