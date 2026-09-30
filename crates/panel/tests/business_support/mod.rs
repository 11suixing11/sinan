#![allow(dead_code)]

use crate::release_support;

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};
use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, Method, Response, StatusCode, header};
use serde_json::{Value, json};
use sinan_panel::{AppState, config::Config, router};
use sinan_protocol::{
    AuthChallenge, AuthResponse, EnrollRequest, Envelope, Hello, HelloAck, PROTOCOL_VERSION,
    StaticInfo,
};
use sqlx::PgPool;
use std::sync::Arc;
use std::{collections::BTreeMap, path::PathBuf, time::Duration};
use tokio::{net::TcpListener, task::JoinHandle, time::timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use uuid::Uuid;

pub type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
const PASSWORD: &str = "business-test-password";

pub struct TestPanel {
    pub state: AppState,
    pub client: Client,
    pub base: String,
    directory: PathBuf,
    task: JoinHandle<()>,
}

impl TestPanel {
    pub async fn start(pool: PgPool) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let listen = listener.local_addr()?;
        let base = format!("http://{listen}");
        let directory = std::env::temp_dir().join(format!("sinan-business-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory)?;
        let directory = directory.canonicalize()?;
        let mut state = AppState::new(
            pool,
            Config {
                database_url: String::new(),
                listen,
                public_url: base.clone(),
                data_dir: directory.clone(),
                admin_password: Some(PASSWORD.into()),
            },
        )
        .await?;
        state.release_keys = Some(Arc::new(release_support::trusted_keys()));
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .expect("test HTTP server")
        });
        Ok(Self {
            state,
            base,
            directory,
            task,
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()?,
        })
    }

    pub async fn admin_cookie(&self) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/api/login", self.base))
            .json(&json!({"password":PASSWORD}))
            .send()
            .await?
            .error_for_status()?;
        Ok(response
            .headers()
            .get(header::SET_COOKIE)
            .context("session header")?
            .to_str()?
            .split(';')
            .next()
            .context("session cookie")?
            .into())
    }

    pub async fn admin(
        &self,
        method: Method,
        path: &str,
        cookie: &str,
        body: Option<Value>,
    ) -> Result<Response> {
        let request = self
            .client
            .request(method, format!("{}{path}", self.base))
            .header(header::COOKIE, cookie);
        Ok(match body {
            Some(body) => request.json(&body),
            None => request,
        }
        .send()
        .await?)
    }

    pub async fn create_server(&self, cookie: &str, name: &str) -> Result<i64> {
        let response = self
            .admin(
                Method::POST,
                "/api/servers",
                cookie,
                Some(json!({"name":name})),
            )
            .await?;
        anyhow::ensure!(
            response.status() == StatusCode::CREATED,
            "server creation failed: {}",
            response.text().await?
        );
        id(&response.json::<Value>().await?)
    }

    pub async fn create_node(&self, cookie: &str, server_id: i64, name: &str) -> Result<Value> {
        self.enable_plugin(cookie, server_id).await?;
        let response = self.admin(Method::POST, "/api/plugins/sing-box/nodes", cookie, Some(json!({"name":name,"server_id":server_id,"public_host":"proxy.example.com","sni":"www.example.com"}))).await?;
        anyhow::ensure!(
            response.status() == StatusCode::CREATED,
            "node creation failed: {}",
            response.text().await?
        );
        Ok(response.json().await?)
    }

    pub async fn enable_plugin(&self, cookie: &str, server_id: i64) -> Result<()> {
        self.admin(
            Method::POST,
            &format!("/api/plugins/sing-box/servers/{server_id}/enable"),
            cookie,
            Some(json!({})),
        )
        .await?
        .error_for_status()?;
        Ok(())
    }

    pub async fn create_user(&self, cookie: &str, name: &str) -> Result<Value> {
        let response = self
            .admin(
                Method::POST,
                "/api/plugins/sing-box/users",
                cookie,
                Some(json!({"name":name})),
            )
            .await?;
        anyhow::ensure!(
            response.status() == StatusCode::CREATED,
            "user creation failed: {}",
            response.text().await?
        );
        Ok(response.json().await?)
    }

    pub async fn grant(&self, cookie: &str, user_id: i64, node_id: i64) -> Result<Value> {
        Ok(self
            .admin(
                Method::POST,
                &format!("/api/plugins/sing-box/users/{user_id}/accesses"),
                cookie,
                Some(json!({"node_id":node_id})),
            )
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn publish_now(&self) -> Result<()> {
        sqlx::query("UPDATE servers SET dirty_at=0 WHERE dirty_at IS NOT NULL")
            .execute(&self.state.pool)
            .await?;
        sinan_panel::publisher::publish_due(&self.state).await
    }

    pub async fn authenticated_device(
        &self,
        cookie: &str,
        name: &str,
    ) -> Result<(i64, Socket, HelloAck)> {
        let (server, socket, ack, _) = self.authenticated_device_with_key(cookie, name).await?;
        Ok((server, socket, ack))
    }

    pub async fn authenticated_device_with_key(
        &self,
        cookie: &str,
        name: &str,
    ) -> Result<(i64, Socket, HelloAck, SigningKey)> {
        let server_id = self.create_server(cookie, name).await?;
        let value: Value = self
            .admin(
                Method::POST,
                &format!("/api/servers/{server_id}/enrollment"),
                cookie,
                None,
            )
            .await?
            .error_for_status()?
            .json()
            .await?;
        let key = SigningKey::generate(&mut rand::rngs::OsRng);
        self.client
            .post(format!("{}/api/agent/v1/enroll", self.base))
            .json(&EnrollRequest {
                token: value["token"].as_str().context("enrollment token")?.into(),
                device_public_key: URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
                static_info: StaticInfo {
                    arch: Some("amd64".into()),
                    hostname: Some("business-test".into()),
                    ..StaticInfo::default()
                },
            })
            .send()
            .await?
            .error_for_status()?;
        let (mut socket, _) = connect_async(format!(
            "{}/api/agent/v1/ws",
            self.base.replacen("http", "ws", 1)
        ))
        .await?;
        let challenge = receive_envelope(&mut socket).await?;
        anyhow::ensure!(
            challenge.message_type == "auth.challenge",
            "missing device challenge"
        );
        let challenge: AuthChallenge = challenge.to_payload()?;
        send_envelope(
            &mut socket,
            Envelope::new(
                "auth.response",
                AuthResponse {
                    server_id,
                    signature: URL_SAFE_NO_PAD
                        .encode(key.sign(challenge.nonce.as_bytes()).to_bytes()),
                },
            )?,
        )
        .await?;
        let ack = receive_envelope(&mut socket).await?;
        anyhow::ensure!(ack.message_type == "hello.ack", "missing device session");
        send_envelope(
            &mut socket,
            Envelope::new(
                "hello",
                Hello {
                    agent_version: "business-test".into(),
                    protocol_version: PROTOCOL_VERSION,
                    capabilities: vec![
                        sinan_protocol::release::ARTIFACT_SIGNATURE_CAPABILITY.into(),
                    ],
                    applied: BTreeMap::new(),
                },
            )?,
        )
        .await?;
        socket
            .send(Message::Ping(b"introduced".to_vec().into()))
            .await?;
        timeout(Duration::from_secs(5), async {
            loop {
                match socket
                    .next()
                    .await
                    .context("WebSocket ended before hello barrier")??
                {
                    Message::Pong(bytes) if bytes.as_ref() == b"introduced" => {
                        return Ok::<_, anyhow::Error>(());
                    }
                    Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await?,
                    Message::Pong(_) => {}
                    other => bail!("unexpected frame before hello barrier: {other:?}"),
                }
            }
        })
        .await??;
        Ok((server_id, socket, ack.to_payload()?, key))
    }
}

impl Drop for TestPanel {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

pub fn id(value: &Value) -> Result<i64> {
    value["id"].as_i64().context("object id")
}

pub async fn send_envelope(socket: &mut Socket, envelope: Envelope) -> Result<()> {
    socket
        .send(Message::Text(serde_json::to_string(&envelope)?.into()))
        .await?;
    Ok(())
}

pub async fn receive_envelope(socket: &mut Socket) -> Result<Envelope> {
    timeout(Duration::from_secs(5), async {
        loop {
            match socket.next().await.context("WebSocket ended")?? {
                Message::Text(text) => return Ok(serde_json::from_str(&text)?),
                Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await?,
                Message::Pong(_) => {}
                other => bail!("expected envelope, got {other:?}"),
            }
        }
    })
    .await?
}
