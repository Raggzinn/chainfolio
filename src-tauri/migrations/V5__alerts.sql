-- Alert rules. `condition` is the JSON of `alerts::Condition`, tagged by kind.
-- `armed` turns off while a level condition (price above, depeg...) keeps
-- holding after it fired, so each crossing notifies once.
CREATE TABLE alert_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    condition TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    armed INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    last_triggered_at INTEGER
);

-- Every time a rule fired. The condition is copied so the history still reads
-- right after the rule is edited or deleted. `value` is a decimal string: the
-- price, the 24h change ratio or the USD amount that moved.
CREATE TABLE alert_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id INTEGER REFERENCES alert_rules (id) ON DELETE SET NULL,
    condition TEXT NOT NULL,
    value TEXT NOT NULL,
    subject TEXT,
    triggered_at INTEGER NOT NULL
);

CREATE INDEX alert_events_by_time ON alert_events (triggered_at DESC);
