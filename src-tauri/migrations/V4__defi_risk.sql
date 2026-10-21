-- DeFi and staking positions. `source` names the adapter and chain that
-- produced a row ("aave-v3:base", "lst"); a refresh replaces a whole source.
-- `data` is the full position as JSON (tokens, receipts, LP range…).
CREATE TABLE positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id INTEGER NOT NULL REFERENCES wallets (id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    chain TEXT NOT NULL,
    protocol TEXT NOT NULL,
    value_usd TEXT,
    -- 1 when the value is already part of `balances` (liquid staking tokens).
    in_balances INTEGER NOT NULL DEFAULT 0,
    data TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX positions_wallet_source ON positions (wallet_id, source);

-- Token security verdicts (GoPlus). Spam tokens are hidden from the portfolio.
CREATE TABLE token_flags (
    asset_id TEXT PRIMARY KEY,
    spam INTEGER NOT NULL,
    -- JSON array of reasons: "honeypot", "highTax", "blacklist", "airdropScam"…
    reasons TEXT NOT NULL,
    checked_at INTEGER NOT NULL
);

-- Open ERC-20 allowances granted by tracked wallets. `amount` is the raw
-- on-chain allowance as a decimal string.
CREATE TABLE approvals (
    wallet_id INTEGER NOT NULL REFERENCES wallets (id) ON DELETE CASCADE,
    chain TEXT NOT NULL,
    token TEXT NOT NULL,
    spender TEXT NOT NULL,
    token_symbol TEXT,
    token_decimals INTEGER,
    spender_name TEXT,
    amount TEXT NOT NULL,
    unlimited INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, chain, token, spender)
);

-- When each wallet's approvals were last scanned, per chain.
CREATE TABLE approval_scans (
    wallet_id INTEGER NOT NULL REFERENCES wallets (id) ON DELETE CASCADE,
    chain TEXT NOT NULL,
    scanned_at INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, chain)
);
