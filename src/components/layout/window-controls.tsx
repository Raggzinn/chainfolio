import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { IS_MAC, IS_TAURI } from "@/lib/platform";
import { cn } from "@/lib/utils";

/**
 * Minimize / maximize / close for Windows and Linux, where the native frame is
 * disabled. macOS keeps its own traffic lights via an overlay title bar.
 */
export function WindowControls() {
  if (!IS_TAURI || IS_MAC) return null;
  const win = getCurrentWindow();

  return (
    <div className="-mr-4 ml-2 flex h-full items-stretch">
      <ControlButton label="Minimize" onClick={() => win.minimize()}>
        <Minus className="size-4" />
      </ControlButton>
      <ControlButton label="Maximize" onClick={() => win.toggleMaximize()}>
        <Square className="size-3.5" />
      </ControlButton>
      <ControlButton label="Close" onClick={() => win.close()} danger>
        <X className="size-4" />
      </ControlButton>
    </div>
  );
}

function ControlButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={cn(
        "flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground",
        danger && "hover:bg-[#e81123] hover:text-primary-foreground",
      )}
    >
      {children}
    </button>
  );
}
