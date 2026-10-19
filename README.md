<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lockup-dark.svg">
    <img alt="Chainfolio" src=".github/assets/lockup-light.svg" width="400">
  </picture>
</p>

<p align="center">
  Inteligência open source para o seu portfólio cripto.<br>
  Carteiras, PnL, risco e impostos em um só lugar — 100% local, somente leitura, sem cadastro.
</p>

<p align="center">
  <a href="https://github.com/Raggzinn/chainfolio/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Raggzinn/chainfolio/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Licença MIT" src="https://img.shields.io/badge/licen%C3%A7a-MIT-1E6BFF"></a>
  <img alt="Plataformas" src="https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-1C1D20">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-19E3A1">
</p>

<p align="center">
  <a href="README.en.md">English</a> ·
  <a href="#funcionalidades">Funcionalidades</a> ·
  <a href="#rodando-localmente">Rodando localmente</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="CONTRIBUTING.md">Contribuindo</a>
</p>

> **Seus dados nunca saem do seu computador.** Sem servidores, sem telemetria e sem pedir chave privada ou seed phrase.

> [!NOTE]
> O Chainfolio está no início do desenvolvimento. Mudanças incompatíveis podem acontecer até a v1.0.

---

## Funcionalidades

- **Multichain** — Ethereum e suas L2s (Arbitrum, Base, Optimism, Polygon), BSC, Avalanche, Solana e Bitcoin via xpub
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

Tudo é **somente leitura**: o Chainfolio nunca assina transações nem pede chaves.

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

- [x] Fundação, tema e estrutura do app
- [x] Dashboard, onboarding e modo demo
- [x] Mood do mercado (Fear & Greed)
- [ ] Atualização automática e backup das configurações
- [ ] Carteiras e saldos (EVM, Solana, Bitcoin)
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
