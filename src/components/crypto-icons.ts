import NetworkArbitrumOne from "@web3icons/react/icons/networks/NetworkArbitrumOne";
import NetworkAvalanche from "@web3icons/react/icons/networks/NetworkAvalanche";
import NetworkBase from "@web3icons/react/icons/networks/NetworkBase";
import NetworkBinanceSmartChain from "@web3icons/react/icons/networks/NetworkBinanceSmartChain";
import NetworkBitcoin from "@web3icons/react/icons/networks/NetworkBitcoin";
import NetworkEthereum from "@web3icons/react/icons/networks/NetworkEthereum";
import NetworkOptimism from "@web3icons/react/icons/networks/NetworkOptimism";
import NetworkPolygon from "@web3icons/react/icons/networks/NetworkPolygon";
import NetworkSolana from "@web3icons/react/icons/networks/NetworkSolana";
import NetworkTron from "@web3icons/react/icons/networks/NetworkTron";
import NetworkXrp from "@web3icons/react/icons/networks/NetworkXrp";
import TokenARB from "@web3icons/react/icons/tokens/TokenARB";
import TokenAVAX from "@web3icons/react/icons/tokens/TokenAVAX";
import TokenBNB from "@web3icons/react/icons/tokens/TokenBNB";
import TokenBTC from "@web3icons/react/icons/tokens/TokenBTC";
import TokenDAI from "@web3icons/react/icons/tokens/TokenDAI";
import TokenETH from "@web3icons/react/icons/tokens/TokenETH";
import TokenLDO from "@web3icons/react/icons/tokens/TokenLDO";
import TokenLINK from "@web3icons/react/icons/tokens/TokenLINK";
import TokenMSOL from "@web3icons/react/icons/tokens/TokenMSOL";
import TokenOP from "@web3icons/react/icons/tokens/TokenOP";
import TokenPOL from "@web3icons/react/icons/tokens/TokenPOL";
import TokenRETH from "@web3icons/react/icons/tokens/TokenRETH";
import TokenSOL from "@web3icons/react/icons/tokens/TokenSOL";
import TokenTRX from "@web3icons/react/icons/tokens/TokenTRX";
import TokenUNI from "@web3icons/react/icons/tokens/TokenUNI";
import TokenUSDC from "@web3icons/react/icons/tokens/TokenUSDC";
import TokenUSDT from "@web3icons/react/icons/tokens/TokenUSDT";
import TokenXRP from "@web3icons/react/icons/tokens/TokenXRP";

/**
 * Curated, statically imported logos so only the icons we use end up in the
 * bundle. Unknown symbols fall back to a monogram in <TokenIcon />.
 */
export const TOKEN_ICONS = {
  ARB: TokenARB,
  AVAX: TokenAVAX,
  // Base has no token; its network mark doubles as the logo.
  BASE: NetworkBase,
  BNB: TokenBNB,
  BTC: TokenBTC,
  DAI: TokenDAI,
  ETH: TokenETH,
  LDO: TokenLDO,
  LINK: TokenLINK,
  MSOL: TokenMSOL,
  OP: TokenOP,
  POL: TokenPOL,
  RETH: TokenRETH,
  SOL: TokenSOL,
  TRX: TokenTRX,
  UNI: TokenUNI,
  USDC: TokenUSDC,
  USDT: TokenUSDT,
  XRP: TokenXRP,
} as const;

export const NETWORK_ICONS = {
  arbitrum: NetworkArbitrumOne,
  avalanche: NetworkAvalanche,
  base: NetworkBase,
  bsc: NetworkBinanceSmartChain,
  bitcoin: NetworkBitcoin,
  ethereum: NetworkEthereum,
  optimism: NetworkOptimism,
  polygon: NetworkPolygon,
  solana: NetworkSolana,
  tron: NetworkTron,
  xrp: NetworkXrp,
} as const;

export type NetworkKey = keyof typeof NETWORK_ICONS;
