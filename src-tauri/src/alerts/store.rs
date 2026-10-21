//! Alert rules and their history in SQLite.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, params};
use rust_decimal::Decimal;
use serde::Serialize;

use super::{Condition, Outcome, Rule};
use crate::{
    db::now_ms,
    error::{Error, Result},
};

/// Balances of every wallet at one moment, to diff around a sync.
#[derive(Debug, Default)]
pub struct Snapshot {
    /// (wallet id, asset id) -> amount.
    pub amounts: HashMap<(i64, String), Decimal>,
    /// Wallets that had synced at least once.
    pub synced: HashSet<i64>,
}

pub fn snapshot(conn: &Connection) -> Result<Snapshot> {
    let mut stmt = conn.prepare("SELECT wallet_id, asset_id, amount FROM balances")?;
    let amounts = stmt
        .query_map([], |r| {
            Ok((
                (r.get::<_, i64>(0)?, r.get::<_, String>(1)?),
                r.get::<_, String>(2)?,
            ))
        })?
        .filter_map(|row| {
            let (key, amount) = row.ok()?;
            Some((key, amount.parse().ok()?))
        })
        .collect();
    let mut stmt = conn.prepare("SELECT id FROM wallets WHERE synced_at IS NOT NULL")?;
    let synced = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Snapshot { amounts, synced })
}

/// USD price per DefiLlama key and per asset id.
pub fn prices(conn: &Connection) -> Result<(HashMap<String, Decimal>, HashMap<String, Decimal>)> {
    let parse = |rows: Vec<(String, String)>| -> HashMap<String, Decimal> {
        rows.into_iter()
            .filter_map(|(k, v)| Some((k, v.parse().ok()?)))
            .collect()
    };
    let mut stmt = conn.prepare("SELECT price_key, price_usd FROM prices")?;
    let by_key = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt = conn.prepare(
        "SELECT a.id, p.price_usd FROM assets a JOIN prices p ON p.price_key = a.price_key",
    )?;
    let by_asset = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok((parse(by_key), parse(by_asset)))
}

fn read_rule(
    r: &rusqlite::Row<'_>,
) -> rusqlite::Result<(i64, String, bool, bool, i64, Option<i64>)> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
    ))
}

pub fn list_rules(conn: &Connection) -> Result<Vec<Rule>> {
    let mut stmt = conn.prepare(
        "SELECT id, condition, enabled, armed, created_at, last_triggered_at
         FROM alert_rules ORDER BY id",
    )?;
    let rows = stmt.query_map([], read_rule)?;
    let mut rules = Vec::new();
    for row in rows {
        let (id, condition, enabled, armed, created_at, last_triggered_at) = row?;
        // A rule written by a newer version may not parse; skip it.
        let Ok(condition) = serde_json::from_str(&condition) else {
            continue;
        };
        rules.push(Rule {
            id,
            condition,
            enabled,
            armed,
            created_at,
            last_triggered_at,
        });
    }
    Ok(rules)
}

fn condition_json(condition: &Condition) -> Result<String> {
    if !condition.validate() {
        return Err(Error::InvalidAlert);
    }
    serde_json::to_string(condition).map_err(|_| Error::InvalidAlert)
}

pub fn insert_rule(conn: &Connection, condition: &Condition, enabled: bool) -> Result<i64> {
    conn.execute(
        "INSERT INTO alert_rules (condition, enabled, created_at) VALUES (?1, ?2, ?3)",
        params![condition_json(condition)?, enabled, now_ms()],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Replaces a rule's condition; it re-arms so the new threshold gets a
/// fresh crossing.
pub fn update_rule(conn: &Connection, id: i64, condition: &Condition) -> Result<()> {
    let changed = conn.execute(
        "UPDATE alert_rules SET condition = ?1, armed = 1 WHERE id = ?2",
        params![condition_json(condition)?, id],
    )?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<()> {
    let changed = conn.execute(
        "UPDATE alert_rules SET enabled = ?1, armed = 1 WHERE id = ?2",
        params![enabled, id],
    )?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub fn delete_rule(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM alert_rules WHERE id = ?1", [id])?;
    Ok(())
}

/// Stores the new armed states and every trigger in one transaction.
pub fn save_outcomes(conn: &mut Connection, outcomes: &[Outcome]) -> Result<()> {
    let now = now_ms();
    let tx = conn.transaction()?;
    for outcome in outcomes {
        tx.execute(
            "UPDATE alert_rules SET armed = ?1 WHERE id = ?2",
            params![outcome.armed, outcome.rule_id],
        )?;
        for trigger in &outcome.triggers {
            tx.execute(
                "INSERT INTO alert_events (rule_id, condition, value, subject, triggered_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    trigger.rule_id,
                    serde_json::to_string(&trigger.condition).map_err(|_| Error::InvalidAlert)?,
                    trigger.value.to_string(),
                    trigger.subject,
                    now
                ],
            )?;
            tx.execute(
                "UPDATE alert_rules SET last_triggered_at = ?1 WHERE id = ?2",
                params![now, trigger.rule_id],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertEvent {
    pub id: i64,
    pub rule_id: Option<i64>,
    pub condition: Condition,
    pub value: Decimal,
    pub subject: Option<String>,
    pub triggered_at: i64,
}

pub fn recent_events(conn: &Connection, limit: u32) -> Result<Vec<AlertEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, rule_id, condition, value, subject, triggered_at FROM alert_events
         ORDER BY triggered_at DESC, id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<i64>>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, i64>(5)?,
        ))
    })?;
    let mut events = Vec::new();
    for row in rows {
        let (id, rule_id, condition, value, subject, triggered_at) = row?;
        let (Ok(condition), Ok(value)) = (serde_json::from_str(&condition), value.parse()) else {
            continue;
        };
        events.push(AlertEvent {
            id,
            rule_id,
            condition,
            value,
            subject,
            triggered_at,
        });
    }
    Ok(events)
}

pub fn clear_events(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM alert_events", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        alerts::{Direction, Trigger},
        db::open_in_memory,
    };

    fn depeg() -> Condition {
        Condition::Depeg {
            ratio: "0.02".parse().unwrap(),
        }
    }

    #[test]
    fn rules_and_triggers_persist() {
        let mut conn = open_in_memory().unwrap();
        let id = insert_rule(&conn, &depeg(), true).unwrap();
        assert!(matches!(
            insert_rule(
                &conn,
                &Condition::Depeg {
                    ratio: Decimal::ZERO
                },
                true
            ),
            Err(Error::InvalidAlert)
        ));

        save_outcomes(
            &mut conn,
            &[Outcome {
                rule_id: id,
                armed: false,
                triggers: vec![Trigger {
                    rule_id: id,
                    condition: depeg(),
                    value: "0.97".parse().unwrap(),
                    subject: Some("USDT".into()),
                }],
            }],
        )
        .unwrap();
        let rule = &list_rules(&conn).unwrap()[0];
        assert!(!rule.armed);
        assert!(rule.last_triggered_at.is_some());

        // Editing re-arms; deleting keeps the history.
        let edited = Condition::PortfolioChange {
            direction: Direction::Down,
            ratio: "0.1".parse().unwrap(),
        };
        update_rule(&conn, id, &edited).unwrap();
        assert!(list_rules(&conn).unwrap()[0].armed);
        delete_rule(&conn, id).unwrap();
        let events = recent_events(&conn, 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].rule_id, None);
        assert_eq!(events[0].condition, depeg());
        assert_eq!(events[0].subject.as_deref(), Some("USDT"));
        assert!(matches!(
            update_rule(&conn, id, &edited),
            Err(Error::NotFound)
        ));
    }
}
