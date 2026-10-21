-- On-chain transactions touching tracked wallets. `id` is "<chain>:<hash>".
CREATE TABLE transactions (
    id TEXT PRIMARY KEY,
    chain TEXT NOT NULL,
    hash TEXT NOT NULL,
    -- Unix time in milliseconds.
    timestamp INTEGER NOT NULL,
    failed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX transactions_by_time ON transactions (timestamp DESC);

-- What one transaction did to one wallet: the auto classification, the user's
-- override and note, and the network fee this wallet paid (if any).
CREATE TABLE tx_entries (
    tx_id TEXT NOT NULL REFERENCES transactions (id) ON DELETE CASCADE,
    wallet_id INTEGER NOT NULL REFERENCES wallets (id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    kind_override TEXT,
    note TEXT,
    fee TEXT,
    fee_asset_id TEXT REFERENCES assets (id),
    PRIMARY KEY (tx_id, wallet_id)
);

CREATE INDEX tx_entries_by_wallet ON tx_entries (wallet_id);

-- Net balance change per asset (received positive, sent negative, fees
-- included), so balances can be replayed back in time.
CREATE TABLE tx_legs (
    tx_id TEXT NOT NULL,
    wallet_id INTEGER NOT NULL,
    asset_id TEXT NOT NULL REFERENCES assets (id),
    amount TEXT NOT NULL,
    counterparty TEXT,
    PRIMARY KEY (tx_id, wallet_id, asset_id),
    FOREIGN KEY (tx_id, wallet_id) REFERENCES tx_entries (tx_id, wallet_id) ON DELETE CASCADE
);

-- Sync progress per wallet and chain: the newest stored hash (where an
-- incremental sync stops) and where the backfill of older history resumes.
CREATE TABLE history_cursors (
    wallet_id INTEGER NOT NULL REFERENCES wallets (id) ON DELETE CASCADE,
    chain TEXT NOT NULL,
    newest TEXT,
    backfill TEXT,
    complete INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, chain)
);

-- Daily USD close per DefiLlama coin id. `day` is the Unix day (ts / 86400).
CREATE TABLE price_history (
    price_key TEXT NOT NULL,
    day INTEGER NOT NULL,
    price_usd TEXT NOT NULL,
    PRIMARY KEY (price_key, day)
);

-- Small key/value store for app settings that live with the data
-- (e.g. optional provider API keys).
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
