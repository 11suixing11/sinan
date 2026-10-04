//! Records what the source migration must not change, so a rehearsal can show
//! that bundles, user subscriptions, grants and the catalog stay the same.
//! A record holds digests and public catalog fields only, never
//! configurations or credentials.

use crate::{AppState, error::ApiResult};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

const SCHEMA: i64 = 1;
const SECTIONS: [&str; 4] = ["bundles", "subscriptions", "external_access", "catalog"];

/// Takes a record at `at`; compare records taken at the same time, because
/// expiry and quotas depend on the clock.
pub async fn snapshot(state: &AppState, at: i64) -> ApiResult<Value> {
    let servers: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM servers WHERE deleted_at IS NULL ORDER BY id")
            .fetch_all(&state.pool)
            .await?;
    let mut bundles = Map::new();
    for server in servers {
        let digest = crate::publisher::bundle_digest(state, server, at).await?;
        bundles.insert(server.to_string(), json!(digest));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let users: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM users WHERE deleted_at IS NULL ORDER BY id")
            .fetch_all(&mut *tx)
            .await?;
    let mut subscriptions = Map::new();
    let mut accesses = Map::new();
    for user in users {
        for format in ["singbox", "links"] {
            let digest = crate::subscriptions::content_digest(&mut tx, user, format).await?;
            subscriptions.insert(format!("{user}/{format}"), json!(digest));
        }
        let nodes = crate::external_access::model::subscription_nodes(&mut tx, user).await?;
        let mut entries = serde_json::to_value(nodes.entries).map_err(anyhow::Error::from)?;
        // A grant's source id is the only field the migration renumbers.
        for entry in entries.as_array_mut().into_iter().flatten() {
            if let Some(entry) = entry.as_object_mut() {
                entry.remove("source_id");
            }
        }
        accesses.insert(user.to_string(), entries);
    }
    let mut catalog = Map::new();
    for mut item in crate::catalog::catalog_on(&mut tx).await? {
        let key = format!(
            "{}:{}",
            item["kind"].as_str().unwrap_or_default(),
            item["id"]
        );
        if item["kind"] == "external"
            && let Some(item) = item.as_object_mut()
        {
            // The concurrency token covers the source id, so it changes too.
            item.remove("source_id");
            item.remove("revision");
        }
        catalog.insert(key, item);
    }
    tx.rollback().await?;
    Ok(json!({
        "schema": SCHEMA,
        "at": at,
        "bundles": bundles,
        "subscriptions": subscriptions,
        "external_access": accesses,
        "catalog": catalog,
    }))
}

/// Keys that differ between two records, as `section/key`; empty when equal.
pub fn compare(before: &Value, after: &Value) -> Vec<String> {
    let mut differences: Vec<String> = ["schema", "at"]
        .into_iter()
        .filter(|field| before[field] != after[field])
        .map(str::to_owned)
        .collect();
    let empty = Map::new();
    for section in SECTIONS {
        let left = before[section].as_object().unwrap_or(&empty);
        let right = after[section].as_object().unwrap_or(&empty);
        let keys: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
        differences.extend(
            keys.into_iter()
                .filter(|key| left.get(*key) != right.get(*key))
                .map(|key| format!("{section}/{key}")),
        );
    }
    differences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_names_each_changed_added_and_removed_key() {
        let before = json!({"schema":1,"at":5,"bundles":{"1":"a","2":"b"},"catalog":{"external:3":{"name":"x"}}});
        let after = json!({"schema":1,"at":5,"bundles":{"1":"a","3":"c"},"catalog":{"external:3":{"name":"y"}}});
        assert_eq!(
            compare(&before, &after),
            ["bundles/2", "bundles/3", "catalog/external:3"]
        );
        assert!(compare(&before, &before).is_empty());
        assert_eq!(
            compare(&before, &json!({"schema":1,"at":6}))
                .first()
                .map(String::as_str),
            Some("at")
        );
    }
}
