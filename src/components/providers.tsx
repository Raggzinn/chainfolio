import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useApplyTheme } from "@/hooks/use-theme";

export function Providers({ children }: { children: React.ReactNode }) {
  useApplyTheme();
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: { queries: { staleTime: 30_000, refetchOnWindowFocus: false } },
      }),
  );

  return (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider delayDuration={200}>
        {children}
        <Toaster richColors position="bottom-right" />
      </TooltipProvider>
    </QueryClientProvider>
  );
}
