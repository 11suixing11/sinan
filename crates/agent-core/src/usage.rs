use crate::state::{read_json, write_json, State};
use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sinan_adapter_sdk::Counter;
use sinan_protocol::{UsageBatch, UsageRecord};
use std::collections::BTreeSet;
use uuid::Uuid;

const LAST_SEQUENCE_KEY: &str = "usage:last_seq";

#[derive(Serialize, Deserialize)]
struct UsageClock {
    epoch: Uuid,
    timestamp: i64,
}

impl State {
    /// Atomically advances cumulative baselines and persists any unsent delta batch.
    pub fn sample_usage(
        &mut self,
        module: &str,
        counters: &[Counter],
        timestamp: i64,
    ) -> Result<Option<UsageBatch>> {
        validate_module(module)?;
        let mut names = BTreeSet::new();
        for counter in counters {
            if counter.stat_name.is_empty()
                || counter.stat_name.len() > 512
                || counter.stat_name.chars().any(char::is_control)
            {
                bail!("invalid counter name");
            }
            if !names.insert(&counter.stat_name) {
                bail!("duplicate counter name in one sample");
            }
        }
        let transaction = self.connection.transaction()?;
        let clock_key = clock_key(module);
        let mut clock: UsageClock = read_json(&transaction, &clock_key)?.unwrap_or(UsageClock {
            epoch: Uuid::new_v4(),
            timestamp,
        });
        if timestamp < clock.timestamp {
            bail!("usage timestamp moved backwards");
        }
        let period_start = clock.timestamp;
        let mut records = Vec::new();
        let mut decreased = false;
        for counter in counters {
            let previous: Option<(String, String)> = transaction.query_row(
                "SELECT uplink, downlink FROM usage_baselines WHERE module = ?1 AND stat_name = ?2",
                params![module, counter.stat_name], |row| Ok((row.get(0)?, row.get(1)?)),
            ).optional()?;
            let (previous_up, previous_down) = previous
                .map(|(up, down)| -> Result<(u64, u64)> {
                    Ok((
                        up.parse().context("invalid saved uplink counter")?,
                        down.parse().context("invalid saved downlink counter")?,
                    ))
                })
                .transpose()?
                .unwrap_or((0, 0));
            let uplink = delta(counter.uplink, previous_up, &mut decreased);
            let downlink = delta(counter.downlink, previous_down, &mut decreased);
            if uplink != 0 || downlink != 0 {
                records.push(UsageRecord {
                    stat_name: counter.stat_name.clone(),
                    uplink,
                    downlink,
                });
            }
        }
        if decreased {
            clock.epoch = Uuid::new_v4();
            transaction.execute(
                "UPDATE usage_baselines SET epoch = ?2 WHERE module = ?1",
                params![module, clock.epoch.to_string()],
            )?;
        }
        for counter in counters {
            transaction.execute(
                "INSERT INTO usage_baselines (module, stat_name, epoch, uplink, downlink, observed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT (module, stat_name) DO UPDATE SET epoch = excluded.epoch, uplink = excluded.uplink, downlink = excluded.downlink, observed_at = excluded.observed_at",
                params![module, counter.stat_name, clock.epoch.to_string(), counter.uplink.to_string(), counter.downlink.to_string(), timestamp],
            )?;
        }
        clock.timestamp = timestamp;
        write_json(&transaction, &clock_key, &clock)?;
        let batch = if records.is_empty() {
            None
        } else {
            records.sort_by(|left, right| left.stat_name.cmp(&right.stat_name));
            let last_seq: u64 = read_json(&transaction, LAST_SEQUENCE_KEY)?.unwrap_or(0);
            let seq = last_seq
                .checked_add(1)
                .context("usage sequence exhausted")?;
            let batch = UsageBatch {
                epoch: clock.epoch,
                seq,
                period_start,
                period_end: timestamp,
                records,
            };
            transaction.execute(
                "INSERT INTO usage_outbox (epoch, seq, batch) VALUES (?1, ?2, ?3)",
                params![
                    batch.epoch.to_string(),
                    seq.to_string(),
                    serde_json::to_string(&batch)?
                ],
            )?;
            write_json(&transaction, LAST_SEQUENCE_KEY, &seq)?;
            Some(batch)
        };
        transaction.commit()?;
        if decreased {
            tracing::warn!(module, "cumulative usage counter decreased; a new epoch was opened and a missing collection window may exist");
        }
        Ok(batch)
    }

    /// Starts a new cycle only when the caller knows the runtime counters reset.
    pub fn begin_usage_epoch(&mut self, module: &str, timestamp: i64) -> Result<()> {
        validate_module(module)?;
        let transaction = self.connection.transaction()?;
        let key = clock_key(module);
        if let Some(previous) = read_json::<UsageClock>(&transaction, &key)? {
            if timestamp < previous.timestamp {
                bail!("usage timestamp moved backwards");
            }
        }
        transaction.execute("DELETE FROM usage_baselines WHERE module = ?1", [module])?;
        write_json(
            &transaction,
            &key,
            &UsageClock {
                epoch: Uuid::new_v4(),
                timestamp,
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn pending_usage(&self) -> Result<Vec<UsageBatch>> {
        let mut statement = self.connection.prepare(
            "SELECT batch FROM usage_outbox WHERE acknowledged = 0 ORDER BY length(seq), seq",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    pub fn acknowledge_usage(&mut self, epoch: Uuid, seq: u64) -> Result<()> {
        self.connection.execute(
            "UPDATE usage_outbox SET acknowledged = 1 WHERE epoch = ?1 AND seq = ?2",
            params![epoch.to_string(), seq.to_string()],
        )?;
        Ok(())
    }

    pub fn pending_usage_count(&self) -> Result<usize> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM usage_outbox WHERE acknowledged = 0",
            [],
            |row| row.get(0),
        )?;
        usize::try_from(count).context("pending usage count out of range")
    }

    pub fn cleanup_acknowledged(&mut self) -> Result<()> {
        self.connection
            .execute("DELETE FROM usage_outbox WHERE acknowledged = 1", [])?;
        Ok(())
    }
}

fn delta(current: u64, previous: u64, decreased: &mut bool) -> u64 {
    match current.checked_sub(previous) {
        Some(value) => value,
        None => {
            *decreased = true;
            current
        }
    }
}

fn clock_key(module: &str) -> String {
    format!("usage:module:{module}")
}

fn validate_module(module: &str) -> Result<()> {
    if module.is_empty() || module.len() > 256 || module.chars().any(char::is_control) {
        bail!("invalid usage module");
    }
    Ok(())
}
