use crate::error::{ApiError, ApiResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use serde::Serialize;
use sinan_compiler::{Access, Node};
use sqlx::{FromRow, Postgres, Transaction};
use x25519_dalek::{PublicKey, StaticSecret};

pub(crate) const NODE_COLUMNS: &str = "n.id, n.name, n.server_id, n.protocol, n.port, n.public_host, n.sni, n.private_key, n.public_key, n.short_id";

#[derive(FromRow)]
pub(crate) struct NodeRow {
    pub id: i64,
    pub name: String,
    pub server_id: i64,
    pub protocol: String,
    pub port: i32,
    pub public_host: String,
    pub sni: String,
    pub private_key: String,
    pub public_key: String,
    pub short_id: String,
}

#[derive(Serialize)]
pub struct NodeView {
    pub id: i64,
    pub name: String,
    pub server_id: i64,
    pub protocol: String,
    pub port: u16,
    pub public_host: String,
    pub sni: String,
    pub public_key: String,
    pub short_id: String,
}

impl NodeRow {
    pub(crate) fn model(&self, users: Vec<Access>) -> anyhow::Result<Node> {
        Ok(Node {
            id: self.id,
            name: self.name.clone(),
            port: self.port.try_into()?,
            public_host: self.public_host.clone(),
            sni: self.sni.clone(),
            private_key: self.private_key.clone(),
            public_key: self.public_key.clone(),
            short_id: self.short_id.clone(),
            users,
        })
    }

    pub(crate) fn view(self) -> ApiResult<NodeView> {
        Ok(NodeView {
            id: self.id,
            name: self.name,
            server_id: self.server_id,
            protocol: self.protocol,
            port: self.port.try_into().map_err(anyhow::Error::from)?,
            public_host: self.public_host,
            sni: self.sni,
            public_key: self.public_key,
            short_id: self.short_id,
        })
    }
}

pub fn generate_reality_keypair() -> (String, String) {
    let secret = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (
        URL_SAFE_NO_PAD.encode(secret.to_bytes()),
        URL_SAFE_NO_PAD.encode(public.as_bytes()),
    )
}

pub(crate) fn short_id() -> String {
    let mut bytes = [0_u8; 4];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn name(value: &str) -> ApiResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 128 || value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "名称需为 1 至 128 个字符，且不能包含控制字符".into(),
        ));
    }
    Ok(value.to_owned())
}

pub(crate) fn validate_node(node: &NodeRow) -> ApiResult<()> {
    sinan_compiler::compile_server(&[node.model(vec![])?])
        .map_err(|error| ApiError::BadRequest(format!("节点配置无效：{error}")))?;
    Ok(())
}

pub(crate) async fn lock_server(
    transaction: &mut Transaction<'_, Postgres>,
    id: i64,
) -> ApiResult<()> {
    let found: Option<i64> =
        sqlx::query_scalar("SELECT id FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **transaction)
            .await?;
    found.ok_or(ApiError::NotFound)?;
    Ok(())
}

pub(crate) async fn lock_user(
    transaction: &mut Transaction<'_, Postgres>,
    id: i64,
) -> ApiResult<()> {
    let found: Option<i64> =
        sqlx::query_scalar("SELECT id FROM users WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **transaction)
            .await?;
    found.ok_or(ApiError::NotFound)?;
    Ok(())
}

pub(crate) async fn lock_user_servers(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: i64,
) -> ApiResult<Vec<i64>> {
    let ids: Vec<i64> = sqlx::query_scalar("SELECT DISTINCT n.server_id FROM accesses a JOIN nodes n ON n.id=a.node_id WHERE a.user_id=$1 ORDER BY n.server_id").bind(user_id).fetch_all(&mut **transaction).await?;
    sqlx::query("SELECT id FROM servers WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(&ids)
        .fetch_all(&mut **transaction)
        .await?;
    Ok(ids)
}

pub(crate) async fn mark_dirty(
    transaction: &mut Transaction<'_, Postgres>,
    server_ids: &[i64],
) -> ApiResult<()> {
    let affected: Vec<i64> = sqlx::query_scalar("SELECT id FROM servers WHERE id=ANY($1) OR id IN (SELECT unnest(ARRAY[n.server_id,e.server_id]) FROM singbox_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id WHERE n.server_id=ANY($1) OR e.server_id=ANY($1)) ORDER BY id FOR UPDATE")
        .bind(server_ids).fetch_all(&mut **transaction).await?;
    sqlx::query("UPDATE servers SET dirty_at=FLOOR(EXTRACT(EPOCH FROM clock_timestamp())*1000)::bigint WHERE id=ANY($1) AND deleted_at IS NULL").bind(&affected).execute(&mut **transaction).await?;
    Ok(())
}
