<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lockup-dark.svg">
    <img alt="Chainfolio" src=".github/assets/lockup-light.svg" width="400">
  </picture>
</h1>

<p align="center">
  Inteligência open source para o seu portfólio cripto.<br>
  Carteiras, PnL, risco e impostos em um só lugar — 100% local, somente leitura, sem cadastro.
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
  <a href="README.md">English</a> ·
  <a href="#funcionalidades">Funcionalidades</a> ·
  <a href="#rodando-localmente">Rodando localmente</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="CONTRIBUTING.md">Contribuindo</a>
</p>

> **Seus dados nunca saem do seu computador.** Sem servidores, sem telemetria e sem pedir chave privada ou seed phrase.


---

## Tecnologias

| Categoria | Stack |
|-----------|-------|
| Desktop | [Tauri 2](https://tauri.app/) (Rust) |
| Frontend | [React 19](https://react.dev/) + [Vite](https://vite.dev/) |
| Linguagem | [TypeScript](https://www.typescriptlang.org/) + [Rust](https://www.rust-lang.org/) |
| Estilização | [Tailwind CSS 4](https://tailwindcss.com/) + [shadcn/ui](https://ui.shadcn.com/) |
| Animações | [GSAP](https://gsap.com/) |
| Dados | [TanStack Query](https://tanstack.com/query) + [TanStack Router](https://tanstack.com/router) |
| Gráficos | [Recharts](https://recharts.org/) |
| Ícones | [Lucide](https://lucide.dev/) + [Web3 Icons](https://github.com/0xa3k5/web3icons) |
| Banco local | SQLite (criptografado com SQLCipher, opcional) |
| Qualidade | [Biome](https://biomejs.dev/) + [Vitest](https://vitest.dev/) + Clippy |

---

## Funcionalidades

- **Multichain** — Ethereum e suas L2s (Arbitrum, Base, Optimism, Polygon), BSC, Avalanche, Solana, Bitcoin via xpub, TRON e XRP Ledger
- **PnL em tempo real** — realizado e não realizado, por FIFO, LIFO ou custo médio
- **Transações unificadas** — histórico único de todas as redes, classificado automaticamente, com detecção de transferências entre suas próprias carteiras
- **Alocação** — por ativo, rede, categoria e protocolo, com benchmark contra BTC e ETH
- **DeFi e staking** — Aave, Compound, Uniswap v3, Lido, Rocket Pool, Marinade, Jito e staking nativo
- **Score de risco** — concentração, exposição a depeg de stablecoins, tokens golpe e aprovações ilimitadas
- **Mood do mercado** — índice Fear & Greed do dia, com histórico de 30 dias
- **Alertas** — preço, variação do portfólio e atividade on-chain como notificações nativas
- **Impostos (opcional)** — relatórios de IR: isenção mensal de R$ 35 mil, ganho de capital e bens e direitos
- **Modo privacidade** — esconde todos os valores com um clique
- **Modo demo** — portfólio de exemplo para explorar o app sem adicionar carteira
- **Tema claro e escuro**, em português e inglês, com moeda base em USD, BRL ou EUR

---

## Redes suportadas

| Rede | Como adicionar | O que é lido |
|------|----------------|--------------|
| Ethereum + L2s, BSC, Avalanche | Endereço `0x…` ou nome ENS | Tokens, transações, posições DeFi, aprovações |
| Solana | Endereço ou nome SNS | Tokens SPL, transações, staking |
| Bitcoin | Endereço ou xpub / ypub / zpub | Saldo e transações de todos os endereços da carteira |
| TRON | Endereço `T…` | TRX e tokens TRC-20 (USDT e outros) |
| XRP Ledger | Endereço `r…` ou X-address | XRP (reserva exibida como bloqueada) e tokens emitidos como RLUSD |

Tudo é **somente leitura**: o Chainfolio nunca assina transações nem pede chaves.

---

## Rodando localmente

**Requisitos**

- [Node.js 24+](https://nodejs.org/) e [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/)
- Pré-requisitos do Tauri para o seu sistema: [Windows, macOS ou Linux](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/Raggzinn/chainfolio.git
cd chainfolio
pnpm install
pnpm tauri dev
```

---

## Comandos

| Comando | Descrição |
|---------|-----------|
| `pnpm tauri dev` | App desktop com hot reload |
| `pnpm dev` | Só o frontend, no navegador |
| `pnpm lint` | Lint e formatação (Biome) |
| `pnpm lint:fix` | Corrige o que for automático |
| `pnpm typecheck` | Checagem de tipos |
| `pnpm test` | Testes do frontend (Vitest) |
| `cargo test` | Testes do Rust (dentro de `src-tauri/`) |
| `pnpm tauri build` | Gera os instaladores |

---

## Estrutura do projeto

```
chainfolio/
├── src/                  # Frontend React
│   ├── components/       # UI compartilhada (shadcn, layout, marca)
│   ├── features/         # Portfólio, mercado, demo...
│   ├── pages/            # Telas do app
│   ├── i18n/             # Traduções pt-BR e en
│   └── stores/           # Preferências do usuário
├── src-tauri/            # Backend Rust (Tauri)
└── .github/              # CI, release e templates
```

---

## Roadmap

> **Status:** início do desenvolvimento — mudanças incompatíveis podem acontecer até a v1.0.

- [x] Fundação, tema e estrutura do app
- [x] Dashboard, onboarding e modo demo
- [x] Mood do mercado (Fear & Greed)
- [ ] Atualização automática e backup das configurações
- [ ] Carteiras e saldos (EVM, Solana, Bitcoin, TRON, XRP Ledger)
- [ ] Transações e histórico do portfólio
- [ ] Motor de PnL (FIFO / LIFO / custo médio)
- [ ] Módulo de impostos (Brasil)
- [ ] Posições DeFi e scanner de risco
- [ ] Alertas, bandeja e sincronização em segundo plano
- [ ] Banco de dados criptografado
- [ ] Release v1.0
- [ ] Corretoras (API somente leitura e importação CSV)

---

## Contribuindo

Contribuições são bem-vindas: novas redes, protocolos DeFi, traduções e melhorias de UI. Leia o [guia de contribuição](CONTRIBUTING.md) — usamos branches por feature, Pull Requests e [Conventional Commits](https://www.conventionalcommits.org/).

Encontrou uma falha de segurança? Veja a [política de segurança](SECURITY.md).

---

## Licença

Distribuído sob a licença [MIT](LICENSE).

---

## Contato

- [GitHub](https://github.com/Raggzinn/)
- [LinkedIn](https://www.linkedin.com/in/joaogoliveirac/)
- [Instagram](https://www.instagram.com/carrneiroo.j/)
