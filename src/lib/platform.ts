import { isTauri } from "@tauri-apps/api/core";

export const IS_TAURI = isTauri();
export const IS_MAC = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);
