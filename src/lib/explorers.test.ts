import { CHAIN_IDS } from "@/lib/chains";
import { txUrl } from "./explorers";

describe("txUrl", () => {
  it("builds an https link for every chain", () => {
    for (const chain of CHAIN_IDS) {
      expect(txUrl(chain, "abc")).toMatch(/^https:\/\/.+abc$/);
    }
  });

  it("uses the chain's own explorer", () => {
    expect(txUrl("solana", "sig")).toBe("https://solscan.io/tx/sig");
    expect(txUrl("tron", "id")).toBe("https://tronscan.org/#/transaction/id");
  });
});
