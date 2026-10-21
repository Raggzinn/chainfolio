-- TRON and XRP Ledger wallets. SQLite can't alter a CHECK constraint, so the
-- wallets table is rebuilt; the family is validated in Rust from now on.
-- Runs with foreign keys off (see db::prepare) so child rows survive the swap.
CREATE TABLE wallets_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL,
    family TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('address', 'xpub')),
    address TEXT NOT NULL,
    input TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    synced_at INTEGER,
    UNIQUE (family, address)
);

INSERT INTO wallets_new (id, label, family, kind, address, input, created_at, synced_at)
SELECT id, label, family, kind, address, input, created_at, synced_at FROM wallets;

DROP TABLE wallets;

ALTER TABLE wallets_new RENAME TO wallets;

-- Part of a balance that can't be spent, e.g. the XRP Ledger account reserve.
ALTER TABLE balances ADD COLUMN locked TEXT;
