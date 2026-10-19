//! Well-known ERC-20 tokens per chain, checked with `balanceOf` wherever no
//! keyless token indexer is reachable. Contracts are lowercase; decimals are
//! verified on-chain by the ignored live tests.

use super::Chain;

pub struct KnownToken {
    pub contract: &'static str,
    pub symbol: &'static str,
    pub name: &'static str,
    pub decimals: u32,
}

const fn t(
    contract: &'static str,
    symbol: &'static str,
    name: &'static str,
    decimals: u32,
) -> KnownToken {
    KnownToken {
        contract,
        symbol,
        name,
        decimals,
    }
}

const ETHEREUM: &[KnownToken] = &[
    t(
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        "USDC",
        "USD Coin",
        6,
    ),
    t(
        "0xdac17f958d2ee523a2206206994597c13d831ec7",
        "USDT",
        "Tether USD",
        6,
    ),
    t(
        "0x6b175474e89094c44da98b954eedeac495271d0f",
        "DAI",
        "Dai",
        18,
    ),
    t(
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        "WETH",
        "Wrapped Ether",
        18,
    ),
    t(
        "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599",
        "WBTC",
        "Wrapped BTC",
        8,
    ),
    t(
        "0xae7ab96520de3a18e5e111b5eaab095312d7fe84",
        "stETH",
        "Lido Staked Ether",
        18,
    ),
    t(
        "0x7f39c581f595b53c5cb19bd0b3f8da6c935e2ca0",
        "wstETH",
        "Wrapped stETH",
        18,
    ),
    t(
        "0xae78736cd615f374d3085123a210448e74fc6393",
        "rETH",
        "Rocket Pool ETH",
        18,
    ),
    t(
        "0x514910771af9ca656af840dff83e8264ecf986ca",
        "LINK",
        "Chainlink",
        18,
    ),
    t(
        "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984",
        "UNI",
        "Uniswap",
        18,
    ),
    t(
        "0x5a98fcbea516cf06857215779fd812ca3bef1b32",
        "LDO",
        "Lido DAO",
        18,
    ),
    t(
        "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9",
        "AAVE",
        "Aave",
        18,
    ),
    t(
        "0x6982508145454ce325ddbe47a25d4ec3d2311933",
        "PEPE",
        "Pepe",
        18,
    ),
    t(
        "0x95ad61b0a150d79219dcf64e1e6cc01f0b64c4ce",
        "SHIB",
        "Shiba Inu",
        18,
    ),
];

const ARBITRUM: &[KnownToken] = &[
    t(
        "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        "USDC",
        "USD Coin",
        6,
    ),
    t(
        "0xff970a61a04b1ca14834a43f5de4533ebddb5cc8",
        "USDC.e",
        "Bridged USDC",
        6,
    ),
    t(
        "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
        "USDT",
        "Tether USD",
        6,
    ),
    t(
        "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1",
        "DAI",
        "Dai",
        18,
    ),
    t(
        "0x82af49447d8a07e3bd95bd0d56f35241523fbab1",
        "WETH",
        "Wrapped Ether",
        18,
    ),
    t(
        "0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f",
        "WBTC",
        "Wrapped BTC",
        8,
    ),
    t(
        "0x5979d7b546e38e414f7e9822514be443a4800529",
        "wstETH",
        "Wrapped stETH",
        18,
    ),
    t(
        "0x912ce59144191c1204e64559fe8253a0e49e6548",
        "ARB",
        "Arbitrum",
        18,
    ),
    t(
        "0xfc5a1a6eb076a2c7ad06ed22c90d7e710e35ad0a",
        "GMX",
        "GMX",
        18,
    ),
    t(
        "0xf97f4df75117a78c1a5a0dbb814af92458539fb4",
        "LINK",
        "Chainlink",
        18,
    ),
];

const BASE: &[KnownToken] = &[
    t(
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        "USDC",
        "USD Coin",
        6,
    ),
    t(
        "0xd9aaec86b65d86f6a7b5b1b0c42ffa531710b6ca",
        "USDbC",
        "Bridged USDC",
        6,
    ),
    t(
        "0x50c5725949a6f0c72e6c4a641f24049a917db0cb",
        "DAI",
        "Dai",
        18,
    ),
    t(
        "0x4200000000000000000000000000000000000006",
        "WETH",
        "Wrapped Ether",
        18,
    ),
    t(
        "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf",
        "cbBTC",
        "Coinbase Wrapped BTC",
        8,
    ),
    t(
        "0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22",
        "cbETH",
        "Coinbase Wrapped Staked ETH",
        18,
    ),
    t(
        "0xc1cba3fcea344f92d9239c08c0568f6f2f0ee452",
        "wstETH",
        "Wrapped stETH",
        18,
    ),
    t(
        "0x940181a94a35a4569e4529a3cdfb74e38fd98631",
        "AERO",
        "Aerodrome",
        18,
    ),
    t(
        "0x4ed4e862860bed51a9570b96d89af5e1b0efefed",
        "DEGEN",
        "Degen",
        18,
    ),
    t(
        "0x532f27101965dd16442e59d40670faf5ebb142e4",
        "BRETT",
        "Brett",
        18,
    ),
];

const OPTIMISM: &[KnownToken] = &[
    t(
        "0x0b2c639c533813f4aa9d7837caf62653d097ff85",
        "USDC",
        "USD Coin",
        6,
    ),
    t(
        "0x7f5c764cbc14f9669b88837ca1490cca17c31607",
        "USDC.e",
        "Bridged USDC",
        6,
    ),
    t(
        "0x94b008aa00579c1307b0ef2c499ad98a8ce58e58",
        "USDT",
        "Tether USD",
        6,
    ),
    t(
        "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1",
        "DAI",
        "Dai",
        18,
    ),
    t(
        "0x4200000000000000000000000000000000000006",
        "WETH",
        "Wrapped Ether",
        18,
    ),
    t(
        "0x68f180fcce6836688e9084f035309e29bf0a2095",
        "WBTC",
        "Wrapped BTC",
        8,
    ),
    t(
        "0x1f32b1c2345538c0c6f582fcb022739c4a194ebb",
        "wstETH",
        "Wrapped stETH",
        18,
    ),
    t(
        "0x4200000000000000000000000000000000000042",
        "OP",
        "Optimism",
        18,
    ),
    t(
        "0x9560e827af36c94d2ac33a39bce1fe78631088db",
        "VELO",
        "Velodrome",
        18,
    ),
    t(
        "0x8700daec35af8ff88c16bdf0418774cb3d7599b4",
        "SNX",
        "Synthetix",
        18,
    ),
];

const POLYGON: &[KnownToken] = &[
    t(
        "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
        "USDC",
        "USD Coin",
        6,
    ),
    t(
        "0x2791bca1f2de4661ed88a30c99a7a9449aa84174",
        "USDC.e",
        "Bridged USDC",
        6,
    ),
    t(
        "0xc2132d05d31c914a87c6611c10748aeb04b58e8f",
        "USDT",
        "Tether USD",
        6,
    ),
    t(
        "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063",
        "DAI",
        "Dai",
        18,
    ),
    t(
        "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619",
        "WETH",
        "Wrapped Ether",
        18,
    ),
    t(
        "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6",
        "WBTC",
        "Wrapped BTC",
        8,
    ),
    t(
        "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270",
        "WPOL",
        "Wrapped POL",
        18,
    ),
    t(
        "0x53e0bca35ec356bd5dddfebbd1fc0fd03fabad39",
        "LINK",
        "Chainlink",
        18,
    ),
    t(
        "0xd6df932a45c0f255f85145f286ea0b292b21c90b",
        "AAVE",
        "Aave",
        18,
    ),
];

const BSC: &[KnownToken] = &[
    t(
        "0x55d398326f99059ff775485246999027b3197955",
        "USDT",
        "Tether USD",
        18,
    ),
    t(
        "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
        "USDC",
        "USD Coin",
        18,
    ),
    t(
        "0xc5f0f7b66764f6ec8c8dff7ba683102295e16409",
        "FDUSD",
        "First Digital USD",
        18,
    ),
    t(
        "0x1af3f329e8be154074d8769d1ffa4ee058b1dbc3",
        "DAI",
        "Dai",
        18,
    ),
    t(
        "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c",
        "WBNB",
        "Wrapped BNB",
        18,
    ),
    t(
        "0x2170ed0880ac9a755fd29b2688956bd959f933f8",
        "ETH",
        "Binance-Peg Ether",
        18,
    ),
    t(
        "0x7130d2a12b9bcbfae4f2634d864a1ee1ce3ead9c",
        "BTCB",
        "Binance-Peg BTC",
        18,
    ),
    t(
        "0x0e09fabb73bd3ade0a17ecc321fd13a19e81ce82",
        "CAKE",
        "PancakeSwap",
        18,
    ),
];

const AVALANCHE: &[KnownToken] = &[
    t(
        "0xb97ef9ef8734c71904d8002f8b6bc66dd9c48a6e",
        "USDC",
        "USD Coin",
        6,
    ),
    t(
        "0x9702230a8ea53601f5cd2dc00fdbc13d4df4a8c7",
        "USDT",
        "Tether USD",
        6,
    ),
    t(
        "0xd586e7f844cea2f87f50152665bcbc2c279d8d70",
        "DAI.e",
        "Bridged Dai",
        18,
    ),
    t(
        "0xb31f66aa3c1e785363f0875a1b74e27b85fd66c7",
        "WAVAX",
        "Wrapped AVAX",
        18,
    ),
    t(
        "0x49d5c2bdffac6ce2bfdb6640f4f80f226bc10bab",
        "WETH.e",
        "Bridged Ether",
        18,
    ),
    t(
        "0x152b9d0fdc40c096757f570a51e494bd4b943e50",
        "BTC.b",
        "Bitcoin (Avalanche bridge)",
        8,
    ),
    t(
        "0x2b2c81e08f1af8835a78bb2a90ae924ace0ea4be",
        "sAVAX",
        "BENQI Staked AVAX",
        18,
    ),
    t(
        "0x6e84a6216ea6dacc71ee8e6b0a5b7322eebc0fdd",
        "JOE",
        "Trader Joe",
        18,
    ),
];

pub fn known_tokens(chain: Chain) -> &'static [KnownToken] {
    match chain {
        Chain::Ethereum => ETHEREUM,
        Chain::Arbitrum => ARBITRUM,
        Chain::Base => BASE,
        Chain::Optimism => OPTIMISM,
        Chain::Polygon => POLYGON,
        Chain::Bsc => BSC,
        Chain::Avalanche => AVALANCHE,
        Chain::Solana | Chain::Bitcoin | Chain::Tron | Chain::Xrp => &[],
    }
}
