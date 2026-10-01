#![forbid(unsafe_code)]

use anyhow::Result;
use serde_json::{Value, json};
use sqlx::{PgPool, migrate::Migrator};
use std::borrow::Cow;
use uuid::Uuid;

#[sqlx::test(migrations = false)]
async fn runtime_extensions_upgrade_the_existing_installation_schema_without_replacing_history(
    pool: PgPool,
) -> Result<()> {
    let migrations = sqlx::migrate!();
    let old = Migrator {
        migrations: Cow::Owned(
            migrations
                .iter()
                .filter(|m| m.version <= 23)
                .cloned()
                .collect(),
        ),
        ..Migrator::DEFAULT
    };
    old.run(&pool).await?;
    let server: i64 =
        sqlx::query_scalar("INSERT INTO servers(name) VALUES('TEST_ONLY upgrade') RETURNING id")
            .fetch_one(&pool)
            .await?;
    sqlx::query("INSERT INTO singbox_installation(server_id,target_rev,error,checked_at) VALUES($1,7,'TEST_ONLY retained failure',123)")
        .bind(server).execute(&pool).await?;
    let pending = Uuid::new_v4();
    let completed = Uuid::new_v4();
    let result = json!({"id":completed,"status":"succeeded","finished_at":20,"stdout":"preserved","stderr":"","timed_out":false,"truncated":false});
    for (id, value) in [(pending, None), (completed, Some(result.clone()))] {
        sqlx::query("INSERT INTO remote_commands(id,server_id,requested_at,spec,result,result_digest) VALUES($1,$2,10,$3,$4,$5)")
            .bind(id).bind(server)
            .bind(json!({"id":id,"command":"TEST_ONLY inert history","timeout_secs":1,"expires_at":100}))
            .bind(value).bind(if id == completed { Some("original-digest") } else { None })
            .execute(&pool).await?;
    }
    migrations.run(&pool).await?;
    migrations.run(&pool).await?;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM remote_commands WHERE id=$1")
            .bind(pending)
            .fetch_one(&pool)
            .await?,
        "claimed"
    );
    let preserved: (String, Value, String) =
        sqlx::query_as("SELECT state,result,result_digest FROM remote_commands WHERE id=$1")
            .bind(completed)
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        preserved,
        ("succeeded".into(), result, "original-digest".into())
    );
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await?;
    assert_eq!(versions, (1..=31).collect::<Vec<_>>());
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT to_regclass('singbox_installation')::text")
            .fetch_one(&pool)
            .await?,
        "singbox_installation"
    );
    let installation: (i64, String, i64) = sqlx::query_as(
        "SELECT target_rev,error,checked_at FROM singbox_installation WHERE server_id=$1",
    )
    .bind(server)
    .fetch_one(&pool)
    .await?;
    assert_eq!(installation, (7, "TEST_ONLY retained failure".into(), 123));
    Ok(())
}
