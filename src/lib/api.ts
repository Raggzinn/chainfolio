import { invoke } from "@tauri-apps/api/core";

/**
 * Typed wrappers around the Rust commands in `src-tauri/src/commands.rs`.
 * Money and amounts arrive as decimal strings (rust_decimal) so no precision
 * is lost over IPC; convert with `Number()` only for display.
 */

export type Decimal = string;

export type Family = "evm" | "solana" | "bitcoin" | "tron" | "xrpl";

export type ChainId =
  | "ethereum"
  | "arbitrum"
  | "base"
  | "optimism"
  | "polygon"
  | "bsc"
  | "avalanche"
  | "solana"
  | "bitcoin"
  | "tron"
  | "xrp";

export type InputKind = "address" | "ens" | "sns" | "xpub";

export interface Detection {
  family: Family;
  kind: InputKind;
  normalized: string;
  chains: ChainId[];
}

export interface WalletSummary {
  id: number;
  label: string;
  family: Family;
  kind: "address" | "xpub";
  address: string;
  input: string;
  createdAt: number;
  syncedAt: number | null;
  totalUsd: Decimal;
  chains: ChainId[];
}

export interface PortfolioAsset {
  id: string;
  chain: ChainId;
  symbol: string;
  name: string;
  logoUrl: string | null;
  amount: Decimal;
  /** Part of `amount` that can't be spent, e.g. the XRP Ledger reserve. */
  locked: Decimal | null;
  priceUsd: Decimal;
  valueUsd: Decimal;
  /** Ratio, 0.05 = +5%. */
  change24h: Decimal | null;
}

export interface ChainTotal {
  chain: ChainId;
  valueUsd: Decimal;
}

export interface Portfolio {
  totalUsd: Decimal;
  change24hUsd: Decimal;
  change24h: Decimal | null;
  assets: PortfolioAsset[];
  chains: ChainTotal[];
  wallets: WalletSummary[];
  unpricedCount: number;
  syncedAt: number | null;
}

export interface SyncIssue {
  walletId: number | null;
  chain: ChainId | null;
  message: string;
}

export interface RefreshReport {
  issues: SyncIssue[];
  portfolio: Portfolio;
}

export interface SecurityStatus {
  /** A master password protects the database. */
  encrypted: boolean;
  /** The database is closed until `unlock` succeeds. */
  locked: boolean;
}

/** Backend settings (`settings.json`) plus the OS launch-at-login flag. */
export interface AppSettings {
  closeToTray: boolean;
  /** 0 turns background refresh off. */
  syncIntervalMinutes: number;
  notifications: boolean;
  locale: string;
  launchAtLogin: boolean;
}

export const ERROR_CODES = [
  "invalidAddress",
  "badChecksum",
  "nameNotFound",
  "duplicate",
  "notFound",
  "emptyLabel",
  "locked",
  "wrongPassword",
  "weakPassword",
  "network",
  "storage",
] as const;
export type ErrorCode = (typeof ERROR_CODES)[number] | "unknown";

export class ApiError extends Error {
  constructor(
    readonly code: ErrorCode,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

/** Normalizes whatever a rejected `invoke` produced into an `ApiError`. */
export function toApiError(error: unknown): ApiError {
  if (error instanceof ApiError) return error;
  if (typeof error === "object" && error !== null && "code" in error) {
    const { code, message } = error as { code: unknown; message?: unknown };
    const known = ERROR_CODES.find((c) => c === code);
    return new ApiError(known ?? "unknown", String(message ?? code));
  }
  return new ApiError("unknown", error instanceof Error ? error.message : String(error));
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toApiError(error);
  }
}

export const api = {
  detectAddress: (input: string) => call<Detection>("detect_address", { input }),
  addWallet: (input: string, label?: string) =>
    call<WalletSummary>("add_wallet", { input, label: label?.trim() || null }),
  listWallets: () => call<WalletSummary[]>("list_wallets"),
  renameWallet: (id: number, label: string) => call<void>("rename_wallet", { id, label }),
  removeWallet: (id: number) => call<void>("remove_wallet", { id }),
  getPortfolio: () => call<Portfolio>("get_portfolio"),
  refreshBalances: (walletId?: number) =>
    call<RefreshReport>("refresh_balances", { walletId: walletId ?? null }),
  securityStatus: () => call<SecurityStatus>("security_status"),
  unlock: (password: string) => call<void>("unlock", { password }),
  lock: () => call<void>("lock"),
  getAppSettings: () => call<AppSettings>("get_app_settings"),
  updateAppSettings: (update: AppSettings) => call<AppSettings>("update_app_settings", { update }),
  setTrayPrivacy: (hidden: boolean) => call<void>("set_tray_privacy", { hidden }),
  /** `newPassword: null` removes the master password. */
  setMasterPassword: (currentPassword: string | null, newPassword: string | null) =>
    call<void>("set_master_password", { currentPassword, newPassword }),
};
