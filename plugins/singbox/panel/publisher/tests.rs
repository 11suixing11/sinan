use super::*;
use crate::config::Config;
use anyhow::Result;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn stale_due_candidate_cannot_create_legacy_enablement_after_capability_disappears(
    pool: PgPool,
) -> Result<()> {
    let state = AppState::new(
        pool.clone(),
        Config {
            database_url: String::new(),
            listen: "127.0.0.1:0".parse()?,
            public_url: "http://127.0.0.1".into(),
            data_dir: std::env::temp_dir(),
            admin_password: Some("TEST_ONLY-plugin-publication-password".into()),
        },
    )
    .await?;
    let server: i64 = sqlx::query_scalar(
        "INSERT INTO servers(name,capabilities,dirty_at) VALUES('Candidate','[\"singbox\"]',0) RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    sqlx::query("INSERT INTO server_plugins(server_id,plugin,source,enabled_at) VALUES($1,'sing-box','agent_capability',0)")
        .bind(server).execute(&pool).await?;
    // The due scan already selected this ID when a new hello removes support.
    sqlx::query("UPDATE servers SET capabilities='[]' WHERE id=$1")
        .bind(server)
        .execute(&pool)
        .await?;
    publish_server(&state, server).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployments WHERE server_id=$1")
        .bind(server)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    let pending: (i64, Option<i64>) =
        sqlx::query_as("SELECT manifest_rev,dirty_at FROM servers WHERE id=$1")
            .bind(server)
            .fetch_one(&pool)
            .await?;
    assert_eq!(pending, (0, Some(0)));

    // An explicit administrator choice remains eligible without device support.
    sqlx::query("UPDATE server_plugins SET source='administrator' WHERE server_id=$1")
        .bind(server)
        .execute(&pool)
        .await?;
    publish_server(&state, server).await?;
    let completed: (i64, Option<i64>) =
        sqlx::query_as("SELECT manifest_rev,dirty_at FROM servers WHERE id=$1")
            .bind(server)
            .fetch_one(&pool)
            .await?;
    assert_eq!(completed, (1, None));
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn stale_candidate_cannot_publish_after_device_leaves_the_plugin(
    pool: PgPool,
) -> anyhow::Result<()> {
    let state = AppState::new(
        pool.clone(),
        Config {
            database_url: String::new(),
            listen: "127.0.0.1:0".parse()?,
            public_url: "http://127.0.0.1".into(),
            data_dir: std::env::temp_dir().join(format!("sinan-publish-{}", uuid::Uuid::new_v4())),
            admin_password: Some("test-only-plugin-publisher".into()),
        },
    )
    .await?;
    let server: i64 = sqlx::query_scalar("INSERT INTO servers(name,capabilities,dirty_at) VALUES('Candidate','[\"singbox\"]',0) RETURNING id").fetch_one(&pool).await?;
    let candidate: Option<i64> = sqlx::query_scalar(&format!("SELECT s.id FROM servers s LEFT JOIN server_plugins p ON p.server_id=s.id AND p.plugin='sing-box' WHERE s.id=$1 AND {DUE} AND ({}) IS NOT NULL",super::super::settings::SOURCE_SQL))
        .bind(server).fetch_optional(&pool).await?;
    assert_eq!(candidate, Some(server));
    // The device declaration changes after candidate selection. Publication
    // must validate the locked server again instead of creating legacy proof.
    sqlx::query("UPDATE servers SET capabilities='[]' WHERE id=$1")
        .bind(server)
        .execute(&pool)
        .await?;
    publish_server(&state, server).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE server_id=$1")
            .bind(server)
            .fetch_one(&pool)
            .await?,
        0
    );
    let pending: (i64, Option<i64>) =
        sqlx::query_as("SELECT manifest_rev,dirty_at FROM servers WHERE id=$1")
            .bind(server)
            .fetch_one(&pool)
            .await?;
    assert_eq!(pending, (0, Some(0)));
    sqlx::query("INSERT INTO server_plugins(server_id,plugin,source,enabled_at) VALUES($1,'sing-box','administrator',0)").bind(server).execute(&pool).await?;
    publish_server(&state, server).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE server_id=$1")
            .bind(server)
            .fetch_one(&pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, Option<i64>>("SELECT dirty_at FROM servers WHERE id=$1")
            .bind(server)
            .fetch_one(&pool)
            .await?,
        None
    );
    Ok(())
}
