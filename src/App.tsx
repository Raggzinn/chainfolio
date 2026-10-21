import { RouterProvider } from "@tanstack/react-router";
import { Providers } from "@/components/providers";
import { SecurityGate } from "@/features/security/lock-screen";
import { router } from "@/router";

export default function App() {
  return (
    <Providers>
      <SecurityGate>
        <RouterProvider router={router} />
      </SecurityGate>
    </Providers>
  );
}
