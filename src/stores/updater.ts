import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { create } from "zustand";
import { IS_TAURI } from "@/lib/platform";

export type UpdaterStatus =
  | "idle"
  | "checking"
  | "upToDate"
  | "available"
  | "downloading"
  | "ready"
  | "error";

interface UpdaterState {
  status: UpdaterStatus;
  update: Update | null;
  /** 0–1 while downloading; null when the size is unknown. */
  progress: number | null;
  error: string | null;
  lastCheckedAt: number | null;
  dismissed: boolean;
  check: (opts?: { silent?: boolean }) => Promise<void>;
  install: () => Promise<void>;
  dismiss: () => void;
}

/**
 * Wraps the Tauri updater so the banner and Settings share one state.
 * Outside the desktop shell (browser dev) every action is a no-op.
 */
export const useUpdater = create<UpdaterState>()((set, get) => ({
  status: "idle",
  update: null,
  progress: null,
  error: null,
  lastCheckedAt: null,
  dismissed: false,

  check: async ({ silent = false } = {}) => {
    if (!IS_TAURI) return;
    const { status } = get();
    if (status === "checking" || status === "downloading") return;
    set({ status: "checking", error: null });
    try {
      const update = await check();
      set({
        update,
        status: update ? "available" : "upToDate",
        dismissed: false,
        lastCheckedAt: Date.now(),
      });
    } catch (e) {
      // Background checks fail quietly (offline, no release yet); manual ones report.
      set({
        status: silent ? "idle" : "error",
        error: e instanceof Error ? e.message : String(e),
        lastCheckedAt: Date.now(),
      });
    }
  },

  install: async () => {
    const { update } = get();
    if (!update) return;
    set({ status: "downloading", progress: 0, error: null });
    let total = 0;
    let received = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          received += event.data.chunkLength;
          set({ progress: total ? Math.min(1, received / total) : null });
        }
        if (event.event === "Finished") set({ progress: 1 });
      });
      set({ status: "ready" });
      await relaunch();
    } catch (e) {
      set({ status: "error", error: e instanceof Error ? e.message : String(e) });
    }
  },

  dismiss: () => set({ dismissed: true }),
}));
