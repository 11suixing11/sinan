use crate::{AppState, agent_api};
use serde_json::Value;
use sinan_protocol::{
    ApplyResult, ApplyStatus, Envelope, RUNTIME_CHECKPOINT_CAPABILITY,
    RUNTIME_RECOVERY_BARRIER_CAPABILITY, RuntimeCheckpoint, RuntimeCheckpointRequest,
    RuntimeRecoveryBarrierRequest, now_timestamp, runtime_module_valid,
};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

mod api;
mod receipts;
mod storage;
pub use api::routes;
pub use receipts::{record_barrier_result, record_checkpoint_result};
use storage::{PendingRequest, current_binding, enqueue};
pub(crate) use storage::{confirmed_is_current, lock_server, target_is_current};

pub async fn checkpoint_required(state: &AppState, server_id: i64) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT checkpoint_required FROM runtime_control_devices WHERE server_id=$1),false)")
        .bind(server_id).fetch_one(&state.pool).await?)
}

/// Once a device uses exact checkpoints, missing capabilities cannot restore legacy trust.
pub async fn register_capabilities(
    state: &AppState,
    server_id: i64,
    capabilities: &[String],
) -> anyhow::Result<()> {
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server_id).await?;
    if capabilities
        .iter()
        .any(|value| value == RUNTIME_CHECKPOINT_CAPABILITY)
    {
        sqlx::query("INSERT INTO runtime_control_devices(server_id,checkpoint_required) VALUES($1,true) ON CONFLICT(server_id) DO UPDATE SET checkpoint_required=true")
            .bind(server_id).execute(&mut *tx).await?;
    }
    let required: bool = sqlx::query_scalar("SELECT COALESCE((SELECT checkpoint_required FROM runtime_control_devices WHERE server_id=$1),false)")
        .bind(server_id).fetch_one(&mut *tx).await?;
    if required {
        // A new connection must confirm its own running instance, including capability loss.
        sqlx::query(
            "UPDATE server_module_status SET healthy=false,updated_at=$2 WHERE server_id=$1",
        )
        .bind(server_id)
        .bind(now_timestamp())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn supports(capabilities: &Value, capability: &str) -> bool {
    capabilities.as_array().is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str() == Some(capability))
    })
}

async fn checkpoint_in_transaction(
    connection: &mut PgConnection,
    server_id: i64,
    module: &str,
) -> anyhow::Result<RuntimeCheckpointRequest> {
    let expected = current_binding(connection, server_id, module).await?;
    let now = now_timestamp();
    let request = RuntimeCheckpointRequest {
        request_id: Uuid::new_v4(),
        expected,
        expires_at: now + 120,
    };
    let payload = enqueue(
        connection,
        server_id,
        PendingRequest {
            module,
            kind: "checkpoint",
            digest: request.digest()?,
            payload: serde_json::to_value(&request)?,
            request_id: request.request_id,
            now,
            expires_at: request.expires_at,
        },
    )
    .await?;
    sqlx::query("UPDATE server_module_status SET healthy=false,updated_at=$3 WHERE server_id=$1 AND module=$2")
        .bind(server_id).bind(module).bind(now).execute(&mut *connection).await?;
    Ok(serde_json::from_value(payload)?)
}

/// The expected identity is always derived from the published target in the database.
pub async fn request_checkpoint(
    state: &AppState,
    server_id: i64,
    module: &str,
) -> anyhow::Result<RuntimeCheckpointRequest> {
    let mut tx = state.pool.begin().await?;
    let capabilities = lock_server(&mut tx, server_id).await?;
    anyhow::ensure!(
        supports(&capabilities, RUNTIME_CHECKPOINT_CAPABILITY),
        "device does not support runtime checkpoints"
    );
    let request = checkpoint_in_transaction(&mut tx, server_id, module).await?;
    tx.commit().await?;
    agent_api::notify(
        state,
        server_id,
        Envelope::new("runtime.checkpoint.request", &request)?,
    )
    .await;
    Ok(request)
}

/// A barrier pins an already verified activation; callers cannot supply a hash or instance.
pub async fn request_barrier(
    state: &AppState,
    server_id: i64,
    module: &str,
    minimum_revision: u64,
) -> anyhow::Result<RuntimeRecoveryBarrierRequest> {
    let mut tx = state.pool.begin().await?;
    let capabilities = lock_server(&mut tx, server_id).await?;
    anyhow::ensure!(
        supports(&capabilities, RUNTIME_CHECKPOINT_CAPABILITY)
            && supports(&capabilities, RUNTIME_RECOVERY_BARRIER_CAPABILITY),
        "device does not support recovery barriers"
    );
    let binding = current_binding(&mut tx, server_id, module).await?;
    anyhow::ensure!(
        minimum_revision > 0 && minimum_revision <= binding.revision,
        "invalid recovery revision floor"
    );
    let row = sqlx::query("SELECT checkpoint_json,minimum_revision FROM runtime_module_checkpoints WHERE server_id=$1 AND module=$2 FOR UPDATE")
        .bind(server_id).bind(module).fetch_optional(&mut *tx).await?
        .ok_or_else(|| anyhow::anyhow!("no verified runtime checkpoint"))?;
    let expected: RuntimeCheckpoint = serde_json::from_value(row.get("checkpoint_json"))?;
    anyhow::ensure!(
        expected.valid()
            && expected.healthy
            && expected.binding == binding
            && confirmed_is_current(&mut tx, server_id, &binding).await?,
        "verified runtime checkpoint is no longer current"
    );
    let minimum_revision = minimum_revision.max(row.get::<i64, _>("minimum_revision") as u64);
    let now = now_timestamp();
    let request = RuntimeRecoveryBarrierRequest {
        request_id: Uuid::new_v4(),
        expected,
        minimum_revision,
        expires_at: now + 120,
    };
    anyhow::ensure!(request.valid_at(now), "invalid recovery revision floor");
    let payload = enqueue(
        &mut tx,
        server_id,
        PendingRequest {
            module,
            kind: "barrier",
            digest: request.digest()?,
            payload: serde_json::to_value(&request)?,
            request_id: request.request_id,
            now,
            expires_at: request.expires_at,
        },
    )
    .await?;
    let request: RuntimeRecoveryBarrierRequest = serde_json::from_value(payload)?;
    tx.commit().await?;
    agent_api::notify(
        state,
        server_id,
        Envelope::new("runtime.barrier.request", &request)?,
    )
    .await;
    Ok(request)
}

/// ApplyResult is evidence of completion, not proof of the exact running configuration.
pub async fn record_apply(
    state: &AppState,
    server_id: i64,
    result: &ApplyResult,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        runtime_module_valid(&result.module) && result.rev > 0 && result.rev <= i64::MAX as u64,
        "invalid apply deployment"
    );
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server_id).await?;
    anyhow::ensure!(
        !result.op_id.is_nil() && (result.status != ApplyStatus::Applied || result.healthy),
        "invalid apply operation identity or status"
    );
    let payload = serde_json::to_value(result)?;
    let existing: Option<Value> = sqlx::query_scalar(
        "SELECT result_json FROM runtime_apply_facts WHERE server_id=$1 AND op_id=$2",
    )
    .bind(server_id)
    .bind(result.op_id)
    .fetch_optional(&mut *tx)
    .await?;
    let duplicate = existing.is_some();
    if let Some(existing) = existing {
        anyhow::ensure!(existing == payload, "conflicting apply operation result");
    } else {
        sqlx::query("INSERT INTO runtime_apply_facts(server_id,op_id,module,rev,result_json,received_at) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(server_id).bind(result.op_id).bind(&result.module).bind(result.rev as i64)
            .bind(payload).bind(now_timestamp()).execute(&mut *tx).await?;
    }
    let target = current_binding(&mut tx, server_id, &result.module).await?;
    if duplicate
        && (result.status != ApplyStatus::Applied
            || target.revision != result.rev
            || confirmed_is_current(&mut tx, server_id, &target).await?)
    {
        tx.commit().await?;
        return Ok(());
    }
    if target.revision == result.rev {
        sqlx::query("UPDATE server_module_status SET last_result_rev=GREATEST(last_result_rev,$3),healthy=false,last_error=$4,updated_at=$5 WHERE server_id=$1 AND module=$2")
            .bind(server_id).bind(&result.module).bind(result.rev as i64)
            .bind(result.error.as_ref().map(|value| value.chars().take(2048).collect::<String>()))
            .bind(now_timestamp()).execute(&mut *tx).await?;
    }
    let request = if result.status == ApplyStatus::Applied && target.revision == result.rev {
        Some(checkpoint_in_transaction(&mut tx, server_id, &result.module).await?)
    } else {
        None
    };
    tx.commit().await?;
    if let Some(request) = request {
        agent_api::notify(
            state,
            server_id,
            Envelope::new("runtime.checkpoint.request", request)?,
        )
        .await;
    }
    Ok(())
}

pub async fn replay_pending(state: &AppState, server_id: i64) -> anyhow::Result<()> {
    let now = now_timestamp();
    sqlx::query("UPDATE runtime_control_requests SET state='expired' WHERE server_id=$1 AND state='pending' AND expires_at<=$2")
        .bind(server_id).bind(now).execute(&state.pool).await?;
    let rows = sqlx::query("SELECT kind,request_json FROM runtime_control_requests WHERE server_id=$1 AND state='pending' ORDER BY created_at,request_id LIMIT 64")
        .bind(server_id).fetch_all(&state.pool).await?;
    for row in rows {
        let kind: String = row.get("kind");
        let payload: Value = row.get("request_json");
        let envelope = if kind == "checkpoint" {
            let request: RuntimeCheckpointRequest = serde_json::from_value(payload)?;
            anyhow::ensure!(
                request.valid_at(now),
                "invalid persisted checkpoint request"
            );
            Envelope::new("runtime.checkpoint.request", request)?
        } else {
            let request: RuntimeRecoveryBarrierRequest = serde_json::from_value(payload)?;
            anyhow::ensure!(request.valid_at(now), "invalid persisted barrier request");
            Envelope::new("runtime.barrier.request", request)?
        };
        agent_api::notify(state, server_id, envelope).await;
    }
    Ok(())
}
