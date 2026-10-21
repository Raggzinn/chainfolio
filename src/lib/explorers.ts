import { openUrl } from "@tauri-apps/plugin-opener";
import type { ChainId } from "@/lib/api";
import { IS_TAURI } from "@/lib/platform";

const TX_URL: Record<ChainId, (hash: string) => string> = {
  ethereum: (h) => `https://etherscan.io/tx/${h}`,
  arbitrum: (h) => `https://arbiscan.io/tx/${h}`,
  base: (h) => `https://basescan.org/tx/${h}`,
  optimism: (h) => `https://optimistic.etherscan.io/tx/${h}`,
  polygon: (h) => `https://polygonscan.com/tx/${h}`,
  bsc: (h) => `https://bscscan.com/tx/${h}`,
  avalanche: (h) => `https://snowtrace.io/tx/${h}`,
  solana: (h) => `https://solscan.io/tx/${h}`,
  bitcoin: (h) => `https://mempool.space/tx/${h}`,
  tron: (h) => `https://tronscan.org/#/transaction/${h}`,
  xrp: (h) => `https://livenet.xrpl.org/transactions/${h}`,
};

export function txUrl(chain: ChainId, hash: string): string {
  return TX_URL[chain](hash);
}

/** Opens an https link in the system browser (the webview never navigates away). */
export async function openExternal(url: string) {
  if (!url.startsWith("https://")) return;
  if (IS_TAURI) await openUrl(url);
  else window.open(url, "_blank", "noopener,noreferrer");
}
