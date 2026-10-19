<div align="center">

# Chainfolio

**Inteligência open source para seu portfólio cripto.**
Acompanhe carteiras, PnL, risco e impostos, 100% local, somente leitura, sem cadastro.

[English](README.md) · [Como contribuir](CONTRIBUTING.md)

</div>

> [!NOTE]
> O Chainfolio está no início do desenvolvimento. Mudanças incompatíveis podem acontecer até a v1.0.

## Por que o Chainfolio

- **Privado por padrão**: seus dados nunca saem do seu computador. Sem servidores, sem telemetria, sem cadastro.
- **Somente leitura**: adicione endereços públicos, nomes ENS/SNS ou xpubs. O Chainfolio nunca pede chave privada ou seed phrase.
- **Fontes de dados gratuitas**: APIs públicas funcionam de primeira; use suas próprias API keys para ter limites maiores.

## Funcionalidades

| | |
|---|---|
| **Multichain** | Ethereum, Arbitrum, Base, Optimism, Polygon, BSC, Avalanche, Solana e Bitcoin (xpub). |
| **PnL em tempo real** | PnL realizado e não realizado por FIFO, LIFO ou custo médio. |
| **Transações unificadas** | Histórico único de todas as redes, classificado automaticamente, com detecção de transferências internas. |
| **Alocação** | Por ativo, rede, categoria e protocolo, com benchmark contra BTC e ETH. |
| **DeFi e staking** | Aave, Compound, Uniswap v3, Lido, Rocket Pool, Marinade, Jito e staking nativo. |
| **Risco** | Concentração, exposição a depeg de stablecoins, tokens golpe e aprovações ilimitadas. |
| **Alertas** | Alertas de preço, portfólio e atividade on-chain como notificações nativas. |
| **Impostos (opcional)** | Relatórios de IR: isenção mensal de R$35 mil, ganho de capital e bens e direitos. |

## Desenvolvimento

Pré-requisitos: [Node.js 24+](https://nodejs.org), [pnpm](https://pnpm.io), [Rust](https://rustup.rs) e os [pré-requisitos do Tauri](https://v2.tauri.app/start/prerequisites/) para o seu sistema.

```bash
pnpm install
pnpm tauri dev
```

Veja a lista completa de comandos no [README em inglês](README.md#development).

## Licença

[MIT](LICENSE)
