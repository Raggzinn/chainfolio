<div align="center">

# Chainfolio

**Open-source crypto portfolio intelligence.**
Track every wallet, PnL, risk and taxes, 100% local, watch-only, no account required.

[![CI](https://github.com/Raggzinn/chainfolio/actions/workflows/ci.yml/badge.svg)](https://github.com/Raggzinn/chainfolio/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-1E6BFF.svg)](LICENSE)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-19E3A1)

[Português](README.pt-BR.md) · [Roadmap](#roadmap) · [Contributing](CONTRIBUTING.md)

</div>

> [!NOTE]
> Chainfolio is in early development. Expect breaking changes until v1.0.

## Why Chainfolio

- **Private by design**: your data never leaves your machine. No servers, no telemetry, no sign-up.
- **Watch-only**: add public addresses, ENS/SNS names or xpubs. Chainfolio never asks for private keys or seed phrases.
- **Free data sources by default**: public APIs work out of the box; bring your own API keys for higher limits.

## Features

| | |
|---|---|
| **Multi-chain** | Ethereum, Arbitrum, Base, Optimism, Polygon, BSC, Avalanche, Solana and Bitcoin (xpub). |
| **Real-time PnL** | Realized and unrealized PnL with FIFO, LIFO or average cost. |
| **Unified transactions** | One history across every chain, auto-classified, with internal transfer detection. |
| **Allocation** | By asset, chain, category and protocol, with benchmarks against BTC and ETH. |
| **DeFi & staking** | Aave, Compound, Uniswap v3, Lido, Rocket Pool, Marinade, Jito and native staking. |
| **Risk insights** | Concentration, stablecoin depeg exposure, scam tokens and unlimited token approvals. |
| **Alerts** | Price, portfolio and on-chain activity alerts as native notifications. |
| **Taxes (optional)** | Brazilian income tax (IR) reports: monthly exemption, capital gains, annual holdings. |

## Tech stack

[Tauri 2](https://tauri.app) (Rust) · React 19 · TypeScript · Vite · Tailwind CSS v4 · shadcn/ui · TanStack Router & Query · SQLite

## Development

Prerequisites: [Node.js 24+](https://nodejs.org), [pnpm](https://pnpm.io), [Rust](https://rustup.rs) and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
pnpm install
pnpm tauri dev
```

| Command | Description |
|---|---|
| `pnpm dev` | Frontend only, in the browser |
| `pnpm tauri dev` | Desktop app with hot reload |
| `pnpm lint` / `pnpm lint:fix` | Biome lint and format |
| `pnpm typecheck` | TypeScript |
| `pnpm test` | Frontend tests (Vitest) |
| `cargo test` | Rust tests (run in `src-tauri/`) |
| `pnpm tauri build` | Production installers |

## Roadmap

- [x] Project foundation, theme and app shell
- [ ] Wallets and balances (EVM, Solana, Bitcoin)
- [ ] Transactions and portfolio history
- [ ] PnL engine (FIFO / LIFO / average cost)
- [ ] Optional tax module (Brazil)
- [ ] DeFi positions and risk scanner
- [ ] Alerts, tray and background sync
- [ ] Encrypted database and backups
- [ ] v1.0 release
- [ ] Centralized exchanges (read-only API and CSV import)

## License

[MIT](LICENSE)
