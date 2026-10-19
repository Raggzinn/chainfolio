-- Watch-only wallets. `address` is the canonical form (checksummed EVM address,
-- resolved ENS/SNS target, Bitcoin address or extended public key); `input` is
-- what the user typed, kept for display.
CREATE TABLE wallets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL,
    family TEXT NOT NULL CHECK (family IN ('evm', 'solana', 'bitcoin')),
    kind TEXT NOT NULL CHECK (kind IN ('address', 'xpub')),
    address TEXT NOT NULL,
    input TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    synced_at INTEGER,
    UNIQUE (family, address)
);

-- Every on-chain address that belongs to a wallet. Plain addresses have a single
-- row; extended public keys also record the derived addresses found in use.
CREATE TABLE wallet_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id INTEGER NOT NULL REFERENCES wallets (id) ON DELETE CASCADE,
    address TEXT NOT NULL,
    derivation TEXT,
    UNIQUE (wallet_id, address)
);

-- One row per token per chain. `id` is "<chain>:<contract>" or "<chain>:native";
-- `price_key` is the DefiLlama coin id used to price it.
CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    chain TEXT NOT NULL,
    contract TEXT,
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    decimals INTEGER NOT NULL,
    price_key TEXT NOT NULL,
    logo_url TEXT
);

-- Latest known balance of each asset per wallet. Amounts are decimal strings.
CREATE TABLE balances (
    wallet_id INTEGER NOT NULL REFERENCES wallets (id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES assets (id),
    amount TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, asset_id)
);

-- Latest USD quote per DefiLlama coin id. `change_24h` is a ratio (0.05 = +5%).
CREATE TABLE prices (
    price_key TEXT PRIMARY KEY,
    price_usd TEXT NOT NULL,
    change_24h TEXT,
    updated_at INTEGER NOT NULL
);
