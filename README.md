<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lockup-dark.svg">
    <img alt="Chainfolio" src=".github/assets/lockup-light.svg" width="400">
  </picture>
</h1>

<p align="center">
  Open-source crypto portfolio intelligence.<br>
  Wallets, PnL, risk and taxes in one place — 100% local, watch-only, no sign-up.
</p>

<p align="center">
  <a href="https://github.com/Raggzinn/chainfolio/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/Raggzinn/chainfolio/ci.yml?branch=main&label=CI&style=flat-square&labelColor=1C1D20"></a>
  <a href="https://github.com/Raggzinn/chainfolio/releases"><img alt="Release" src="https://img.shields.io/github/v/release/Raggzinn/chainfolio?include_prereleases&label=release&color=1E6BFF&style=flat-square&labelColor=1C1D20"></a>
  <a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-1E6BFF?style=flat-square&labelColor=1C1D20"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-19E3A1?style=flat-square&labelColor=1C1D20">
  <a href="https://github.com/Raggzinn/chainfolio/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/Raggzinn/chainfolio?color=19E3A1&style=flat-square&labelColor=1C1D20"></a>
</p>

<p align="center">
  <a href="https://tauri.app"><img alt="Tauri 2" src="https://img.shields.io/badge/Tauri%202-24C8D8?style=flat-square&labelColor=1C1D20&logo=tauri&logoColor=F5F7FA"></a>
  <a href="https://www.rust-lang.org"><img alt="Rust" src="https://img.shields.io/badge/Rust-CE422B?style=flat-square&labelColor=1C1D20&logo=rust&logoColor=F5F7FA"></a>
  <a href="https://react.dev"><img alt="React 19" src="https://img.shields.io/badge/React%2019-149ECA?style=flat-square&labelColor=1C1D20&logo=react&logoColor=F5F7FA"></a>
  <a href="https://www.typescriptlang.org"><img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-3178C6?style=flat-square&labelColor=1C1D20&logo=typescript&logoColor=F5F7FA"></a>
  <a href="https://vite.dev"><img alt="Vite" src="https://img.shields.io/badge/Vite-646CFF?style=flat-square&labelColor=1C1D20&logo=vite&logoColor=F5F7FA"></a>
  <a href="https://tailwindcss.com"><img alt="Tailwind CSS 4" src="https://img.shields.io/badge/Tailwind%20CSS%204-06B6D4?style=flat-square&labelColor=1C1D20&logo=tailwindcss&logoColor=F5F7FA"></a>
  <a href="https://ui.shadcn.com"><img alt="shadcn/ui" src="https://img.shields.io/badge/shadcn%2Fui-1C1D20?style=flat-square&labelColor=1C1D20&logo=shadcnui&logoColor=F5F7FA"></a>
  <a href="https://gsap.com"><img alt="GSAP" src="https://img.shields.io/badge/GSAP-0AE448?style=flat-square&labelColor=1C1D20&logo=greensock&logoColor=F5F7FA"></a>
  <a href="https://sqlite.org"><img alt="SQLite" src="https://img.shields.io/badge/SQLite-0F80CC?style=flat-square&labelColor=1C1D20&logo=sqlite&logoColor=F5F7FA"></a>
</p>

<p align="center">
  <a href="README.pt-BR.md">Português</a> ·
  <a href="#features">Features</a> ·
  <a href="#running-locally">Running locally</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

> **Your data never leaves your computer.** No servers, no telemetry, and never a private key or seed phrase.


---

## Tech stack

| Category | Stack |
|----------|-------|
| Desktop | [Tauri 2](https://tauri.app/) (Rust) |
| Frontend | [React 19](https://react.dev/) + [Vite](https://vite.dev/) |
| Language | [TypeScript](https://www.typescriptlang.org/) + [Rust](https://www.rust-lang.org/) |
| Styling | [Tailwind CSS 4](https://tailwindcss.com/) + [shadcn/ui](https://ui.shadcn.com/) |
| Animation | [GSAP](https://gsap.com/) |
| Data | [TanStack Query](https://tanstack.com/query) + [TanStack Router](https://tanstack.com/router) |
| Charts | [Recharts](https://recharts.org/) |
| Icons | [Lucide](https://lucide.dev/) + [Web3 Icons](https://github.com/0xa3k5/web3icons) |
| Local database | SQLite (optionally encrypted with SQLCipher) |
| Quality | [Biome](https://biomejs.dev/) + [Vitest](https://vitest.dev/) + Clippy |

---

## Features

- **Multi-chain** — Ethereum and its L2s (Arbitrum, Base, Optimism, Polygon), BSC, Avalanche, Solana and Bitcoin via xpub
- **Real-time PnL** — realized and unrealized, with FIFO, LIFO or average cost
- **Unified transactions** — one history across every chain, auto-classified, with transfers between your own wallets detected
- **Allocation** — by asset, chain, category and protocol, benchmarked against BTC and ETH
- **DeFi & staking** — Aave, Compound, Uniswap v3, Lido, Rocket Pool, Marinade, Jito and native staking
- **Risk score** — concentration, stablecoin depeg exposure, scam tokens and unlimited approvals
- **Market mood** — today's Fear & Greed index with 30 days of history
- **Alerts** — price, portfolio and on-chain activity as native notifications
- **Taxes (optional)** — Brazilian income tax reports: monthly exemption, capital gains and annual holdings
- **Privacy mode** — hides every value with one click
- **Demo mode** — a sample portfolio to explore the app before adding a wallet
- **Light and dark themes**, in English and Portuguese, with USD, BRL or EUR as base currency

---

## Supported chains

| Chain | How to add | What is read |
|-------|------------|--------------|
| Ethereum + L2s, BSC, Avalanche | `0x…` address or ENS name | Tokens, transactions, DeFi positions, approvals |
| Solana | Address or SNS name | SPL tokens, transactions, staking |
| Bitcoin | Address or xpub / ypub / zpub | Balance and transactions of every wallet address |

Everything is **watch-only**: Chainfolio never signs transactions or asks for keys.

---

## Running locally

**Requirements**

- [Node.js 24+](https://nodejs.org/) and [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/)
- Tauri prerequisites for your OS: [Windows, macOS or Linux](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/Raggzinn/chainfolio.git
cd chainfolio
pnpm install
pnpm tauri dev
```

---

## Commands

| Command | Description |
|---------|-------------|
| `pnpm tauri dev` | Desktop app with hot reload |
| `pnpm dev` | Frontend only, in the browser |
| `pnpm lint` | Lint and format check (Biome) |
| `pnpm lint:fix` | Apply automatic fixes |
| `pnpm typecheck` | Type checking |
| `pnpm test` | Frontend tests (Vitest) |
| `cargo test` | Rust tests (inside `src-tauri/`) |
| `pnpm tauri build` | Build installers |

---

## Project structure

```
chainfolio/
├── src/                  # React frontend
│   ├── components/       # Shared UI (shadcn, layout, brand)
│   ├── features/         # Portfolio, market, demo...
│   ├── pages/            # App screens
│   ├── i18n/             # en and pt-BR translations
│   └── stores/           # User preferences
├── src-tauri/            # Rust backend (Tauri)
└── .github/              # CI, release and templates
```

---

## Roadmap

> **Status:** early development — expect breaking changes until v1.0.

- [x] Foundation, theme and app shell
- [x] Dashboard, onboarding and demo mode
- [x] Market mood (Fear & Greed)
- [ ] Auto-update and settings backup
- [ ] Wallets and balances (EVM, Solana, Bitcoin)
- [ ] Transactions and portfolio history
- [ ] PnL engine (FIFO / LIFO / average cost)
- [ ] Tax module (Brazil)
- [ ] DeFi positions and risk scanner
- [ ] Alerts, tray and background sync
- [ ] Encrypted database
- [ ] v1.0 release
- [ ] Centralized exchanges (read-only API and CSV import)

---

## Contributing

Contributions are welcome: new chains, DeFi protocols, translations and UI polish. Read the [contributing guide](CONTRIBUTING.md) — we use feature branches, Pull Requests and [Conventional Commits](https://www.conventionalcommits.org/).

Found a security issue? See the [security policy](SECURITY.md).

---

## License

Distributed under the [MIT](LICENSE) license.

---

## Contact

- [GitHub](https://github.com/Raggzinn/)
- [LinkedIn](https://www.linkedin.com/in/joaogoliveirac/)
- [Instagram](https://www.instagram.com/carrneiroo.j/)
