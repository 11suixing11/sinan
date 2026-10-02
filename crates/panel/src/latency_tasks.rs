use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sinan_protocol::{ProbeAuthorization, ProbeSpec, now_timestamp};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    spec: ProbeSpec,
    authorization: Option<ProbeAuthorization>,
    default_enabled: bool,
    server_ids: Vec<i64>,
    revision: Option<i64>,
}

#[derive(Serialize)]
pub struct Task {
    id: Uuid,
    spec: ProbeSpec,
    authorization: Option<ProbeAuthorization>,
    default_enabled: bool,
    server_ids: Vec<i64>,
    revision: i64,
}

// Serialize default assignment and group edits before taking server row locks.
pub(crate) async fn lock(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(1936289389, 1)")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

type TaskListRow = (Uuid, Value, bool, i64, Vec<i64>, Option<Value>);

pub async fn list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Vec<Task>>> {
    auth::require_admin(&state, &headers).await?;
    let rows: Vec<TaskListRow> = sqlx::query_as(
        "SELECT t.id,t.spec,t.default_enabled,t.revision,ARRAY(SELECT p.server_id FROM network_probes p
         JOIN servers s ON s.id=p.server_id AND s.deleted_at IS NULL WHERE p.task_id=t.id ORDER BY p.server_id)
         ,t.target_authorization FROM latency_tasks t ORDER BY t.spec->>'name',t.id")
        .fetch_all(&state.pool).await?;
    let tasks = rows
        .into_iter()
        .map(
            |(id, spec, default_enabled, revision, server_ids, authorization)| {
                Ok(Task {
                    id,
                    spec: serde_json::from_value(spec)?,
                    authorization: authorization.map(serde_json::from_value).transpose()?,
                    default_enabled,
                    server_ids,
                    revision,
                })
            },
        )
        .collect::<Result<Vec<_>, serde_json::Error>>()
        .map_err(anyhow::Error::from)?;
    Ok(Json(tasks))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> ApiResult<(StatusCode, Json<Task>)> {
    auth::require_admin(&state, &headers).await?;
    let task = save(&state, Uuid::new_v4(), input, false).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<Input>,
) -> ApiResult<Json<Task>> {
    auth::require_admin(&state, &headers).await?;
    Ok(Json(save(&state, id, input, true).await?))
}

async fn save(state: &AppState, id: Uuid, mut input: Input, editing: bool) -> ApiResult<Task> {
    input.spec.id = id;
    input.spec.name = input.spec.name.trim().into();
    input.spec.target = input.spec.target.trim().into();
    input.spec.carrier = input.spec.carrier.trim().into();
    input.server_ids.sort_unstable();
    if !input.spec.valid()
        || input.server_ids.len() > 4096
        || input.server_ids.iter().any(|id| *id <= 0)
        || input.server_ids.windows(2).any(|ids| ids[0] == ids[1])
    {
        return Err(ApiError::BadRequest(
            "延迟任务配置无效，请检查目标、间隔及服务器选择".into(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    lock(&mut tx).await?;
    let revision = if editing {
        let (previous, revision): (Value, i64) =
            sqlx::query_as("SELECT spec,revision FROM latency_tasks WHERE id=$1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(ApiError::NotFound)?;
        if input.revision != Some(revision) {
            return Err(ApiError::Conflict("任务已被修改，请刷新后重试".into()));
        }
        let previous: ProbeSpec = serde_json::from_value(previous).map_err(anyhow::Error::from)?;
        if previous.kind != input.spec.kind
            || previous.target != input.spec.target
            || previous.port != input.spec.port
        {
            return Err(ApiError::Conflict(
                "检测方式、目标和端口创建后不可修改，请新建任务以保留历史归属".into(),
            ));
        }
        revision
            .checked_add(1)
            .ok_or_else(|| ApiError::Conflict("任务修订号已到上限，请保留历史并新建任务".into()))?
    } else {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM latency_tasks")
            .fetch_one(&mut *tx)
            .await?;
        if count >= 32 {
            return Err(ApiError::Conflict("最多配置 32 个统一延迟任务".into()));
        }
        1
    };
    let mut affected: Vec<i64> =
        sqlx::query_scalar("SELECT server_id FROM network_probes WHERE task_id=$1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
    affected.extend(&input.server_ids);
    affected.sort_unstable();
    affected.dedup();
    sqlx::query("SELECT id FROM servers WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(&affected)
        .fetch_all(&mut *tx)
        .await?;
    let servers: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM servers WHERE id=ANY($1) AND deleted_at IS NULL ORDER BY id",
    )
    .bind(&input.server_ids)
    .fetch_all(&mut *tx)
    .await?;
    if servers != input.server_ids {
        return Err(ApiError::BadRequest(
            "选择的服务器不存在或已删除，请刷新后重试".into(),
        ));
    }
    for server in &servers {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM network_probes WHERE server_id=$1 AND (task_id IS NULL OR task_id<>$2)")
            .bind(server).bind(id).fetch_one(&mut *tx).await?;
        if count >= 32 {
            return Err(ApiError::Conflict(format!(
                "服务器 #{server} 已有 32 个拨测，未保存任何更改"
            )));
        }
    }
    crate::probes::validate_authorization(input.spec.enabled, input.authorization.as_ref())?;
    let authorization = input
        .authorization
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(anyhow::Error::from)?;
    sqlx::query("INSERT INTO latency_tasks(id,spec,default_enabled,revision,target_authorization) VALUES($1,$2,$3,$4,$5)
        ON CONFLICT(id) DO UPDATE SET spec=EXCLUDED.spec,default_enabled=EXCLUDED.default_enabled,revision=EXCLUDED.revision,target_authorization=EXCLUDED.target_authorization")
        .bind(id).bind(json!(input.spec)).bind(input.default_enabled).bind(revision).bind(&authorization).execute(&mut *tx).await?;
    // Removing an assignment never reuses its measurement ID if assigned again later.
    sqlx::query("DELETE FROM network_probes WHERE task_id=$1 AND NOT(server_id=ANY($2))")
        .bind(id)
        .bind(&servers)
        .execute(&mut *tx)
        .await?;
    for server in &servers {
        let previous: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM network_probes WHERE task_id=$1 AND server_id=$2")
                .bind(id)
                .bind(server)
                .fetch_optional(&mut *tx)
                .await?;
        let mut spec = input.spec.clone();
        spec.id = previous.unwrap_or_else(Uuid::new_v4);
        sqlx::query("INSERT INTO network_probes(id,server_id,spec,task_id,target_authorization) VALUES($1,$2,$3,$4,$5) ON CONFLICT(id) DO UPDATE SET spec=EXCLUDED.spec,target_authorization=EXCLUDED.target_authorization,revision=network_probes.revision+1")
            .bind(spec.id).bind(server).bind(json!(spec)).bind(id).bind(&authorization).execute(&mut *tx).await?;
    }
    for server in affected {
        crate::probes::bump_revision(&mut tx, server).await?;
    }
    tx.commit().await?;
    Ok(Task {
        id,
        spec: input.spec,
        authorization: input.authorization,
        default_enabled: input.default_enabled,
        server_ids: servers,
        revision,
    })
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    input: Option<Json<crate::probes::DeleteProbe>>,
) -> ApiResult<StatusCode> {
    auth::require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    lock(&mut tx).await?;
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM latency_tasks WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::NotFound)?;
    if input.and_then(|Json(input)| input.revision) != Some(revision) {
        return Err(ApiError::Conflict("任务已被修改，请刷新后重试".into()));
    }
    let servers: Vec<i64> = sqlx::query_scalar(
        "SELECT server_id FROM network_probes WHERE task_id=$1 ORDER BY server_id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query("SELECT id FROM servers WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(&servers)
        .fetch_all(&mut *tx)
        .await?;
    if sqlx::query("DELETE FROM latency_tasks WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        == 0
    {
        return Err(ApiError::NotFound);
    }
    for server in servers {
        crate::probes::bump_revision(&mut tx, server).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// The caller acquires the group lock before inserting the server and explicit probes.
pub(crate) async fn assign_defaults(
    tx: &mut Transaction<'_, Postgres>,
    server: i64,
) -> ApiResult<()> {
    let defaults: Vec<(Uuid, Value, Option<Value>)> = sqlx::query_as(
        "SELECT id,spec,target_authorization FROM latency_tasks WHERE default_enabled ORDER BY id",
    )
    .fetch_all(&mut **tx)
    .await?;
    let defaults: Vec<_> = defaults
        .into_iter()
        .filter_map(|(id, value, authorization)| {
            let spec: ProbeSpec = serde_json::from_value(value).ok()?;
            let authorization = authorization
                .and_then(|value| serde_json::from_value::<ProbeAuthorization>(value).ok());
            (spec.valid()
                && (!spec.enabled
                    || authorization
                        .as_ref()
                        .is_some_and(|authorization| authorization.allows(now_timestamp()))))
            .then_some((id, spec, authorization))
        })
        .collect();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM network_probes WHERE server_id=$1")
        .bind(server)
        .fetch_one(&mut **tx)
        .await?;
    if count + defaults.len() as i64 > 32 {
        return Err(ApiError::Conflict(
            "初始拨测与默认延迟任务合计超过 32 个，请减少目标后重试".into(),
        ));
    }
    let changed = !defaults.is_empty();
    for (id, mut spec, authorization) in defaults {
        spec.id = Uuid::new_v4();
        sqlx::query("INSERT INTO network_probes(id,server_id,spec,task_id,target_authorization) VALUES($1,$2,$3,$4,$5)")
            .bind(spec.id)
            .bind(server)
            .bind(json!(spec))
            .bind(id)
            .bind(authorization.map(serde_json::to_value).transpose().map_err(anyhow::Error::from)?)
            .execute(&mut **tx)
            .await?;
        sqlx::query("UPDATE latency_tasks SET revision=revision+1 WHERE id=$1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if changed {
        crate::probes::bump_revision(tx, server).await?;
    }
    Ok(())
}
