import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import i18n from "@/i18n";
import type { Detection, Portfolio, WalletSummary } from "@/lib/api";
import { AddWalletDialog } from "./add-wallet-dialog";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
  isTauri: () => true,
}));

const ens: Detection = {
  family: "evm",
  kind: "ens",
  normalized: "vitalik.eth",
  chains: ["ethereum", "arbitrum", "base", "optimism", "polygon", "bsc", "avalanche"],
};

const wallet: WalletSummary = {
  id: 1,
  label: "vitalik.eth",
  family: "evm",
  kind: "address",
  address: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
  input: "vitalik.eth",
  createdAt: 0,
  syncedAt: null,
  totalUsd: "0",
  chains: [],
};

const emptyPortfolio: Portfolio = {
  totalUsd: "0",
  change24hUsd: "0",
  change24h: null,
  assets: [],
  chains: [],
  wallets: [wallet],
  unpricedCount: 0,
  syncedAt: null,
};

function setup() {
  const onOpenChange = vi.fn();
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <AddWalletDialog open onOpenChange={onOpenChange} />
    </QueryClientProvider>,
  );
  return { onOpenChange, user: userEvent.setup() };
}

beforeEach(async () => {
  invoke.mockReset();
  await i18n.changeLanguage("en");
});

describe("AddWalletDialog", () => {
  it("detects the chain, adds the wallet and loads its balances", async () => {
    invoke.mockImplementation(async (command: string) => {
      switch (command) {
        case "detect_address":
          return ens;
        case "add_wallet":
          return wallet;
        case "get_portfolio":
          return emptyPortfolio;
        case "refresh_balances":
          return { issues: [], portfolio: emptyPortfolio };
        default:
          throw new Error(`unexpected ${command}`);
      }
    });
    const { user, onOpenChange } = setup();
    const add = screen.getByRole("button", { name: "Add wallet" });
    expect(add).toBeDisabled();

    await user.type(screen.getByLabelText("Address, name or xpub"), "vitalik.eth");
    expect(await screen.findByText("ENS name")).toBeInTheDocument();
    expect(screen.getByText(/Scans 7 chains/)).toBeInTheDocument();

    await user.type(screen.getByLabelText("Name"), "Vitalik");
    await user.click(add);

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
    expect(invoke).toHaveBeenCalledWith("add_wallet", { input: "vitalik.eth", label: "Vitalik" });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("refresh_balances", { walletId: wallet.id }),
    );
  });

  it("explains why an input is rejected", async () => {
    invoke.mockRejectedValue({ code: "badChecksum", message: "address checksum does not match" });
    const { user } = setup();

    await user.type(screen.getByLabelText("Address, name or xpub"), "0xABCdef");
    expect(await screen.findByRole("alert")).toHaveTextContent("Checksum mismatch");
    expect(screen.getByRole("button", { name: "Add wallet" })).toBeDisabled();
  });

  it("shows backend errors from adding, like duplicates", async () => {
    invoke.mockImplementation(async (command: string) => {
      if (command === "detect_address") return ens;
      throw { code: "duplicate", message: "this wallet is already being tracked" };
    });
    const { user, onOpenChange } = setup();

    await user.type(screen.getByLabelText("Address, name or xpub"), "vitalik.eth");
    await screen.findByText("ENS name");
    await user.click(screen.getByRole("button", { name: "Add wallet" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("already being tracked");
    expect(onOpenChange).not.toHaveBeenCalled();
  });
});
