# Contributing to Chainfolio

Thanks for helping! Bug reports, new chains, DeFi adapters, translations and UI polish are all welcome.

## Getting started

1. Fork and clone the repository.
2. Install the prerequisites listed in the [README](README.md#development).
3. `pnpm install` and `pnpm tauri dev`.

## Workflow

- Create a branch from `main` named after the change type: `feat/solana-staking`, `fix/evm-decimals`, `docs/readme`.
- Keep commits small and focused, following [Conventional Commits](https://www.conventionalcommits.org):

  ```
  feat(wallets): resolve ENS names when adding a wallet
  fix(pnl): include gas fees in cost basis
  ```

  Common types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `style`, `chore`, `ci`, `build`.
- Open a pull request against `main`. The PR title must also follow Conventional Commits; it becomes the squash commit message.
- CI must be green: lint, typecheck, frontend tests, `cargo fmt`, `cargo clippy` and `cargo test`.

## Code guidelines

- **Money is never a float.** All financial math lives in Rust and uses `rust_decimal`.
- The frontend only displays data; business logic belongs in `src-tauri/src/engine`.
- Every user-facing string goes through i18n. Add keys to both `en.json` and `pt-BR.json` (a test checks they match).
- New data providers implement the provider traits and must work without an API key, or degrade gracefully.
- Never log or persist private keys, seed phrases or API keys in plain text.

## Adding a translation

Copy `src/i18n/locales/en.json` to `<locale>.json`, translate it and register it in `src/i18n/index.ts`.
