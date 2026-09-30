#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};
use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, Response, StatusCode, header};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sinan_panel::{AppState, config::Config, router};
use sinan_protocol::{
    AuthChallenge, AuthResponse, EnrollRequest, EnrollResponse, Envelope, Hello, HelloAck,
    Manifest, PROTOCOL_VERSION, StaticInfo,
};
use sqlx::PgPool;
use std::{collections::BTreeMap, path::PathBuf, time::Duration};
use tokio::{net::TcpListener, task::JoinHandle, time::timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use uuid::Uuid;

const PASSWORD: &str = "foundation-test-password";
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct TestPanel {
    state: AppState,
    client: Client,
    base: String,
    directory: PathBuf,
    task: JoinHandle<()>,
}

impl TestPanel {
    async fn start(pool: PgPool) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let listen = listener.local_addr()?;
        let base = format!("http://{listen}");
        let directory = std::env::temp_dir().join(format!("sinan-foundation-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory)?;
        let config = Config {
            database_url: String::new(),
            listen,
            public_url: base.clone(),
            data_dir: directory.clone(),
            admin_password: Some(PASSWORD.into()),
        };
        let state = AppState::new(pool, config).await?;
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("test HTTP server");
        });
        Ok(Self {
            state,
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()?,
            base,
            directory,
            task,
        })
    }

    async fn login(&self, password: &str) -> Result<Response> {
        Ok(self
            .client
            .post(format!("{}/api/login", self.base))
            .json(&json!({"password": password}))
            .send()
            .await?)
    }

    async fn admin_cookie(&self) -> Result<String> {
        let response = self.login(PASSWORD).await?;
        assert!(response.status().is_success());
        session_cookie(&response)
    }

    async fn create_server(&self, cookie: &str, name: &str) -> Result<i64> {
        let response = self
            .client
            .post(format!("{}/api/servers", self.base))
            .header(header::COOKIE, cookie)
            .json(&json!({"name": name}))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::CREATED);
        let server: Value = response.json().await?;
        server["id"].as_i64().context("server id")
    }

    async fn token(&self, cookie: &str, server_id: i64) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/api/servers/{server_id}/enrollment", self.base))
            .header(header::COOKIE, cookie)
            .send()
            .await?;
        assert!(response.status().is_success());
        let value: Value = response.json().await?;
        assert!(
            value["expires_at"].as_i64().context("token expiry")? > sinan_protocol::now_timestamp()
        );
        let token = value["token"].as_str().context("enrollment token")?;
        assert!(
            value["install_command"]
                .as_str()
                .context("install command")?
                .contains(token)
        );
        Ok(token.to_string())
    }

    async fn enroll(&self, request: &EnrollRequest) -> Result<Response> {
        Ok(self
            .client
            .post(format!("{}/api/agent/v1/enroll", self.base))
            .json(request)
            .send()
            .await?)
    }

    async fn connect(&self) -> Result<(Socket, AuthChallenge)> {
        let url = format!("{}/api/agent/v1/ws", self.base.replacen("http", "ws", 1));
        let (mut socket, _) = connect_async(url).await?;
        let envelope = receive_envelope(&mut socket).await?;
        assert_eq!(envelope.message_type, "auth.challenge");
        Ok((socket, envelope.to_payload()?))
    }

    async fn authenticated_device(
        &self,
        cookie: &str,
        name: &str,
    ) -> Result<(i64, Socket, HelloAck)> {
        let server_id = self.create_server(cookie, name).await?;
        let key = SigningKey::generate(&mut rand::rngs::OsRng);
        self.enroll(&enrollment(self.token(cookie, server_id).await?, &key))
            .await?
            .error_for_status()?;
        let (mut socket, challenge) = self.connect().await?;
        send_envelope(
            &mut socket,
            Envelope::new(
                "auth.response",
                signed_response(server_id, &challenge.nonce, &key),
            )?,
        )
        .await?;
        let ack = receive_envelope(&mut socket).await?;
        assert_eq!(ack.message_type, "hello.ack");
        send_envelope(
            &mut socket,
            Envelope::new(
                "hello",
                Hello {
                    agent_version: "foundation-test".into(),
                    protocol_version: PROTOCOL_VERSION,
                    capabilities: vec![],
                    applied: BTreeMap::new(),
                },
            )?,
        )
        .await?;
        Ok((server_id, socket, ack.to_payload()?))
    }
}

impl Drop for TestPanel {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn session_cookie(response: &Response) -> Result<String> {
    response
        .headers()
        .get(header::SET_COOKIE)
        .context("session Set-Cookie header")?
        .to_str()?
        .split(';')
        .next()
        .map(str::to_owned)
        .context("session cookie")
}

fn enrollment(token: String, key: &SigningKey) -> EnrollRequest {
    EnrollRequest {
        token,
        device_public_key: URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()),
        static_info: StaticInfo {
            arch: Some("amd64".into()),
            hostname: Some("foundation-test".into()),
            ..StaticInfo::default()
        },
    }
}

fn signed_response(server_id: i64, nonce: &str, key: &SigningKey) -> AuthResponse {
    AuthResponse {
        server_id,
        signature: URL_SAFE_NO_PAD.encode(key.sign(nonce.as_bytes()).to_bytes()),
    }
}

async fn send_envelope(socket: &mut Socket, envelope: Envelope) -> Result<()> {
    socket
        .send(Message::Text(serde_json::to_string(&envelope)?.into()))
        .await?;
    Ok(())
}

async fn receive_envelope(socket: &mut Socket) -> Result<Envelope> {
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

async fn expect_rejected(socket: &mut Socket) -> Result<()> {
    timeout(Duration::from_secs(5), async {
        while let Some(message) = socket.next().await {
            match message {
                Ok(Message::Close(_)) | Err(_) => return Ok(()),
                Ok(Message::Text(text)) => bail!("rejected authentication returned data: {text}"),
                _ => {}
            }
        }
        Ok(())
    })
    .await?
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_sessions_are_scoped_expiring_and_not_reset_on_restart(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    for path in [
        "/api/me",
        "/api/servers",
        "/api/artifacts",
        "/api/agent/v1/manifest",
    ] {
        assert_eq!(
            panel
                .client
                .get(format!("{}{path}", panel.base))
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
    assert_eq!(
        panel.login("incorrect-password").await?.status(),
        StatusCode::UNAUTHORIZED
    );
    let response = panel.login(PASSWORD).await?;
    assert!(response.status().is_success());
    let attributes = response.headers()[header::SET_COOKIE].to_str()?;
    assert!(attributes.contains("HttpOnly"));
    assert!(attributes.contains("SameSite=Strict"));
    assert!(attributes.contains("Path=/"));
    let cookie = session_cookie(&response)?;
    assert!(
        panel
            .client
            .get(format!("{}/api/me", panel.base))
            .header(header::COOKIE, &cookie)
            .send()
            .await?
            .status()
            .is_success()
    );
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/agent/v1/manifest", panel.base))
            .header(header::COOKIE, &cookie)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let mut restarted_config = (*panel.state.config).clone();
    restarted_config.admin_password = Some("replacement-must-not-reset-password".into());
    AppState::new(pool.clone(), restarted_config).await?;
    assert!(panel.login(PASSWORD).await?.status().is_success());
    assert_eq!(
        panel
            .login("replacement-must-not-reset-password")
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );

    sqlx::query("UPDATE sessions SET expires_at = 0 WHERE admin_id IS NOT NULL")
        .execute(&pool)
        .await?;
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/me", panel.base))
            .header(header::COOKIE, &cookie)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let cookie = panel.admin_cookie().await?;
    assert!(
        panel
            .client
            .post(format!("{}/api/logout", panel.base))
            .header(header::COOKIE, &cookie)
            .send()
            .await?
            .status()
            .is_success()
    );
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/me", panel.base))
            .header(header::COOKIE, &cookie)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn server_crud_and_enrollment_consumption_are_atomic(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let server_id = panel.create_server(&cookie, "Before rename").await?;
    let url = format!("{}/api/servers/{server_id}", panel.base);
    assert!(
        panel
            .client
            .patch(&url)
            .header(header::COOKIE, &cookie)
            .json(&json!({"name": "After rename"}))
            .send()
            .await?
            .status()
            .is_success()
    );
    let server: Value = panel
        .client
        .get(&url)
        .header(header::COOKIE, &cookie)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(server["name"], "After rename");
    let token = panel.token(&cookie, server_id).await?;
    let first_key = SigningKey::generate(&mut rand::rngs::OsRng);
    let second_key = SigningKey::generate(&mut rand::rngs::OsRng);
    let first_request = enrollment(token.clone(), &first_key);
    let second_request = enrollment(token, &second_key);
    let (first, second) = tokio::join!(panel.enroll(&first_request), panel.enroll(&second_request));
    let first = first?;
    let second = second?;
    assert_ne!(first.status().is_success(), second.status().is_success());
    let (winner, rejected) = if first.status().is_success() {
        (first, second)
    } else {
        (second, first)
    };
    assert!(rejected.status().is_client_error());
    assert_eq!(winner.json::<EnrollResponse>().await?.server_id, server_id);
    assert!(
        panel
            .enroll(&first_request)
            .await?
            .status()
            .is_client_error()
    );
    assert!(
        panel
            .client
            .delete(&url)
            .header(header::COOKIE, &cookie)
            .send()
            .await?
            .status()
            .is_success()
    );
    assert_eq!(
        panel
            .client
            .get(&url)
            .header(header::COOKIE, &cookie)
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_keys_do_not_consume_tokens_and_expired_tokens_cannot_enroll(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let server_id = panel.create_server(&cookie, "Invalid key case").await?;
    let key = SigningKey::generate(&mut rand::rngs::OsRng);
    let mut request = enrollment(panel.token(&cookie, server_id).await?, &key);
    let public_key = request.device_public_key.clone();
    request.device_public_key = URL_SAFE_NO_PAD.encode([0_u8; 31]);
    assert_eq!(
        panel.enroll(&request).await?.status(),
        StatusCode::BAD_REQUEST
    );
    request.device_public_key = URL_SAFE_NO_PAD.encode([0_u8; 32]);
    assert_eq!(
        panel.enroll(&request).await?.status(),
        StatusCode::BAD_REQUEST
    );
    request.device_public_key = public_key;
    assert_eq!(
        panel
            .enroll(&request)
            .await?
            .error_for_status()?
            .json::<EnrollResponse>()
            .await?
            .server_id,
        server_id
    );

    let expired_server = panel.create_server(&cookie, "Expired token case").await?;
    let expired_request = enrollment(panel.token(&cookie, expired_server).await?, &key);
    sqlx::query("UPDATE enrollment_tokens SET expires_at = 0 WHERE server_id = $1")
        .bind(expired_server)
        .execute(&pool)
        .await?;
    assert!(
        panel
            .enroll(&expired_request)
            .await?
            .status()
            .is_client_error()
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn bundle_download_preserves_bytes_and_cannot_cross_server_identity(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let (server_id, _socket, ack) = panel.authenticated_device(&cookie, "Bundle owner").await?;
    let other_id = panel.create_server(&cookie, "Other bundle owner").await?;
    let bundle = "{\n  \"files\": {\"config.json\": \"{}\\n\"}\n}\n";
    let digest = format!("{:x}", Sha256::digest(bundle.as_bytes()));
    for (owner, rev) in [(server_id, 1_i64), (other_id, 99_i64)] {
        sqlx::query("INSERT INTO deployments(server_id,module,rev,bundle,bundle_sha256,created_at) VALUES($1,'singbox',$2,$3,$4,$5)")
            .bind(owner).bind(rev).bind(bundle).bind(&digest)
            .bind(sinan_protocol::now_timestamp()).execute(&pool).await?;
    }
    let own_url = format!("{}/api/agent/v1/bundles/1", panel.base);
    assert_eq!(
        panel.client.get(&own_url).send().await?.status(),
        StatusCode::UNAUTHORIZED
    );
    let response = panel
        .client
        .get(&own_url)
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?;
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let bytes = response.bytes().await?;
    assert_eq!(bytes.as_ref(), bundle.as_bytes());
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), digest);
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/agent/v1/bundles/99", panel.base))
            .bearer_auth(&ack.session_token)
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    panel
        .client
        .delete(format!("{}/api/servers/{server_id}", panel.base))
        .header(header::COOKIE, &cookie)
        .send()
        .await?
        .error_for_status()?;
    assert_eq!(
        panel
            .client
            .get(&own_url)
            .bearer_auth(&ack.session_token)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn websocket_challenges_are_connection_bound_and_sessions_expire(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let server_id = panel.create_server(&cookie, "Authenticated agent").await?;
    let key = SigningKey::generate(&mut rand::rngs::OsRng);
    panel
        .enroll(&enrollment(panel.token(&cookie, server_id).await?, &key))
        .await?
        .error_for_status()?;

    let (mut first, first_challenge) = panel.connect().await?;
    let (mut replay, second_challenge) = panel.connect().await?;
    assert_ne!(first_challenge.nonce, second_challenge.nonce);
    let response = signed_response(server_id, &first_challenge.nonce, &key);
    send_envelope(&mut replay, Envelope::new("auth.response", &response)?).await?;
    expect_rejected(&mut replay).await?;

    let (mut unknown, challenge) = panel.connect().await?;
    send_envelope(
        &mut unknown,
        Envelope::new(
            "auth.response",
            signed_response(i64::MAX, &challenge.nonce, &key),
        )?,
    )
    .await?;
    expect_rejected(&mut unknown).await?;

    send_envelope(&mut first, Envelope::new("auth.response", response)?).await?;
    let ack = receive_envelope(&mut first).await?;
    assert_eq!(ack.message_type, "hello.ack");
    let ack: HelloAck = ack.to_payload()?;
    assert_eq!(ack.session_expires_at - ack.server_time, 3600);
    send_envelope(
        &mut first,
        Envelope::new(
            "hello",
            Hello {
                agent_version: "foundation-test".into(),
                protocol_version: PROTOCOL_VERSION,
                capabilities: vec![],
                applied: BTreeMap::new(),
            },
        )?,
    )
    .await?;
    let manifest: Manifest = panel
        .client
        .get(format!("{}/api/agent/v1/manifest", panel.base))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(manifest.rev, 0);
    assert!(manifest.modules.is_empty());
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/me", panel.base))
            .bearer_auth(&ack.session_token)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );

    send_envelope(
        &mut first,
        Envelope::new("future.unknown", json!({"extension": true}))?,
    )
    .await?;
    first
        .send(Message::Ping(b"still-connected".as_slice().into()))
        .await?;
    timeout(Duration::from_secs(5), async {
        loop {
            match first
                .next()
                .await
                .context("authenticated WebSocket ended")??
            {
                Message::Ping(bytes) => first.send(Message::Pong(bytes)).await?,
                Message::Pong(bytes) if bytes.as_ref() == b"still-connected" => break,
                other => bail!("expected heartbeat pong, got {other:?}"),
            }
        }
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    sqlx::query("UPDATE sessions SET expires_at = 0 WHERE server_id = $1")
        .bind(server_id)
        .execute(&pool)
        .await?;
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/agent/v1/manifest", panel.base))
            .bearer_auth(&ack.session_token)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn bootstrap_downloads_require_live_tokens_and_verified_contained_artifacts(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let server_id = panel.create_server(&cookie, "Bootstrap device").await?;
    let token = panel.token(&cookie, server_id).await?;
    let binary = b"test-agent-artifact";
    let hash = format!("{:x}", Sha256::digest(binary));
    let artifact_dir = panel.directory.join("artifacts/agent/0.1.0");
    std::fs::create_dir_all(&artifact_dir)?;
    std::fs::write(artifact_dir.join("amd64"), binary)?;
    std::fs::write(artifact_dir.join("SHA256SUMS"), format!("{hash}  amd64\n"))?;
    let bootstrap_url = format!("{}/api/bootstrap/0.1.0/amd64", panel.base);
    assert!(
        panel
            .client
            .get(&bootstrap_url)
            .send()
            .await?
            .status()
            .is_client_error()
    );
    assert_eq!(
        panel
            .client
            .get(&bootstrap_url)
            .query(&[("token", "invalid")])
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = panel
        .client
        .get(&bootstrap_url)
        .query(&[("token", &token)])
        .send()
        .await?
        .error_for_status()?;
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.bytes().await?.as_ref(), binary);

    let install = panel
        .client
        .get(format!("{}/install.sh", panel.base))
        .query(&[("token", &token)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    assert!(install.contains(&panel.base));
    assert!(install.contains(&token));
    assert!(install.contains(&hash));
    assert!(!install.contains("@@"));
    assert_eq!(
        panel
            .client
            .get(format!(
                "{}/api/bootstrap/%2e%2e%2fsecret/amd64",
                panel.base
            ))
            .query(&[("token", &token)])
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        panel
            .client
            .get(format!(
                "{}/api/agent/v1/artifacts/agent/0.1.0/amd64",
                panel.base
            ))
            .query(&[("token", &token)])
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );

    #[cfg(unix)]
    {
        let outside = panel.directory.join("outside-artifact-root");
        std::fs::write(&outside, binary)?;
        std::os::unix::fs::symlink(&outside, artifact_dir.join("arm64"))?;
        std::fs::write(
            artifact_dir.join("SHA256SUMS"),
            format!("{hash}  amd64\n{hash}  arm64\n"),
        )?;
        assert_eq!(
            panel
                .client
                .get(format!("{}/api/bootstrap/0.1.0/arm64", panel.base))
                .query(&[("token", &token)])
                .send()
                .await?
                .status(),
            StatusCode::NOT_FOUND
        );
    }

    std::fs::write(artifact_dir.join("amd64"), b"corrupted-test-artifact")?;
    assert_eq!(
        panel
            .client
            .get(&bootstrap_url)
            .query(&[("token", &token)])
            .send()
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    std::fs::write(artifact_dir.join("amd64"), binary)?;
    let key = SigningKey::generate(&mut rand::rngs::OsRng);
    panel
        .enroll(&enrollment(token.clone(), &key))
        .await?
        .error_for_status()?;
    assert_eq!(
        panel
            .client
            .get(&bootstrap_url)
            .query(&[("token", &token)])
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        panel
            .client
            .get(format!("{}/install.sh", panel.base))
            .query(&[("token", &token)])
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let (_, _socket, ack) = panel
        .authenticated_device(&cookie, "Authenticated download")
        .await?;
    let response = panel
        .client
        .get(format!(
            "{}/api/agent/v1/artifacts/agent/0.1.0/amd64",
            panel.base
        ))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?;
    assert_eq!(response.bytes().await?.as_ref(), binary);
    Ok(())
}
