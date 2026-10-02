use super::*;
use anyhow::Result;
use sqlx::PgPool;

async fn endpoint(tx: &mut Transaction<'_, Postgres>, enabled: bool) -> Result<i64> {
    let server: i64 = sqlx::query_scalar(
        "INSERT INTO servers(name) VALUES('TEST_ONLY legacy endpoint') RETURNING id",
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(sqlx::query_scalar("INSERT INTO nodes(name,server_id,port,public_host,sni,private_key,public_key,short_id,enabled) VALUES('TEST_ONLY node',$1,443,'node.example.com','www.example.com','TEST_ONLY private','TEST_ONLY public','1234abcd',$2) RETURNING id")
        .bind(server).bind(enabled).fetch_one(&mut **tx).await?)
}
async fn legacy(tx: &mut Transaction<'_, Postgres>, entry: i64, exit: i64) -> Result<i64> {
    let id = sqlx::query_scalar("INSERT INTO singbox_chains(name,entry_node_id,exit_node_id,relay_uuid) VALUES('TEST_ONLY legacy',$1,$2,$3) RETURNING id")
        .bind(entry).bind(exit).bind(Uuid::new_v4()).fetch_one(&mut **tx).await?;
    super::super::mixed_paths::seed_legacy_on(tx, id).await?;
    seed_legacy_projection_on(tx, id).await?;
    Ok(id)
}

#[sqlx::test(migrations = "./migrations")]
async fn newly_created_legacy_chain_preserves_both_snapshot_namespaces_and_disabled_projection(
    pool: PgPool,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let entry = endpoint(&mut tx, true).await?;
    let exit = endpoint(&mut tx, false).await?;
    let id = legacy(&mut tx, entry, exit).await?;
    let value: Chain = sqlx::query_as(&format!("{SELECT} AND c.id=$1"))
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    assert!(!value.available);
    let numeric: serde_json::Value = sqlx::query_scalar(
        "SELECT path_json FROM singbox_chain_versions WHERE chain_id=$1 AND generation=1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let ordered = super::super::ordered_paths::storage::version(&mut tx, id, 1).await?;
    assert!(ordered.legacy);
    assert_eq!(
        numeric["hops"][0]["identity"],
        serde_json::to_value(ordered.snapshot.legacy_relay_uuid)?
    );
    assert_eq!(ordered.snapshot.entry.node.id, entry);
    let super::super::ordered_paths::models::FrozenHop::Managed { endpoint, .. } =
        &ordered.snapshot.hops[0]
    else {
        panic!("legacy hop must remain managed")
    };
    assert_eq!(endpoint.node.id, exit);
    assert!(!endpoint.node.enabled);
    assert_eq!(numeric["hops"][0]["endpoint"]["enabled"], false);
    let generations: (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT active_generation,applied_generation FROM singbox_chains WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(generations, (Some(1), Some(1)));
    tx.commit().await?;
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn node_deletion_guard_retains_policy_numeric_and_ordered_cleanup_owners(
    pool: PgPool,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let target = endpoint(&mut tx, true).await?;
    let entry = endpoint(&mut tx, true).await?;
    super::super::nodes::ensure_unreferenced_on(&mut tx, target).await?;
    let group: i64 = sqlx::query_scalar("INSERT INTO singbox_policy_groups(name) VALUES('TEST_ONLY private name must not leak') RETURNING id").fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO singbox_policy_nodes(group_id,node_id) VALUES($1,$2)")
        .bind(group)
        .bind(target)
        .execute(&mut *tx)
        .await?;
    assert!(matches!(
        super::super::nodes::ensure_unreferenced_on(&mut tx, target).await,
        Err(ApiError::Conflict(_))
    ));
    sqlx::query("DELETE FROM singbox_policy_nodes WHERE group_id=$1")
        .bind(group)
        .execute(&mut *tx)
        .await?;
    let id = legacy(&mut tx, entry, target).await?;
    assert!(matches!(
        super::super::nodes::ensure_unreferenced_on(&mut tx, target).await,
        Err(ApiError::Conflict(_))
    ));
    sqlx::query(
        "UPDATE singbox_chains SET path_kind='mixed',exit_node_id=NULL,relay_uuid=NULL WHERE id=$1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    assert!(matches!(
        super::super::nodes::ensure_unreferenced_on(&mut tx, target).await,
        Err(ApiError::Conflict(_))
    ));
    sqlx::query(
        "UPDATE singbox_chains SET path_kind='ordered',phase='retiring',deleted_at=1 WHERE id=$1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    assert!(matches!(
        super::super::nodes::ensure_unreferenced_on(&mut tx, target).await,
        Err(ApiError::Conflict(_))
    ));
    sqlx::query("UPDATE singbox_chains SET phase='retired',applied_generation=NULL WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    super::super::nodes::ensure_unreferenced_on(&mut tx, target).await?;
    tx.rollback().await?;
    Ok(())
}
