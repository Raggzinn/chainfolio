import { RouterProvider } from "@tanstack/react-router";
import { Providers } from "@/components/providers";
import { router } from "@/router";

export default function App() {
  return (
    <Providers>
      <RouterProvider router={router} />
    </Providers>
  );
}
