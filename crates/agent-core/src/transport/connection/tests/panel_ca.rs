use super::*;
use crate::panel_tls::test_support::{CertificateKind, Material};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use tokio::task::JoinHandle;

struct WssServer {
    origin: String,
    task: JoinHandle<Result<bool>>,
}

impl WssServer {
    async fn start(material: &Material, kind: CertificateKind, key: VerifyingKey) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let origin = format!("https://{}", listener.local_addr()?);
        let acceptor = material.acceptor(&kind)?;
        let task = tokio::spawn(async move {
            let (stream, _) = timeout(Duration::from_secs(5), listener.accept()).await??;
            let Ok(Ok(stream)) = timeout(Duration::from_secs(5), acceptor.accept(stream)).await
            else {
                return Ok(false);
            };
            let Ok(Ok(mut peer)) = timeout(Duration::from_secs(5), accept_async(stream)).await
            else {
                return Ok(false);
            };
            let nonce = "TEST_ONLY_private_panel_challenge";
            peer.send(Frame::Text(
                serde_json::to_string(&Envelope::new(
                    "auth.challenge",
                    AuthChallenge {
                        nonce: nonce.into(),
                        server_time: 10,
                    },
                )?)?
                .into(),
            ))
            .await?;
            let response = timeout(Duration::from_secs(5), peer.next())
                .await?
                .context("TEST_ONLY WSS peer closed before authentication")??;
            let Frame::Text(response) = response else {
                anyhow::bail!("TEST_ONLY WSS authentication envelope missing");
            };
            let response: Envelope = serde_json::from_str(&response)?;
            assert_eq!(response.v, PROTOCOL_VERSION);
            let Message::AuthResponse(response) = response.decode()? else {
                anyhow::bail!("TEST_ONLY WSS authentication response missing");
            };
            assert_eq!(response.server_id, 7);
            key.verify(
                nonce.as_bytes(),
                &Signature::from_slice(&URL_SAFE_NO_PAD.decode(response.signature)?)?,
            )?;
            peer.send(Frame::Text(
                serde_json::to_string(&Envelope::new(
                    "hello.ack",
                    HelloAck {
                        server_time: 10,
                        session_token: "TEST_ONLY_private_panel_session".into(),
                        session_expires_at: 3_610,
                    },
                )?)?
                .into(),
            ))
            .await?;
            // Finish the actual close exchange so no detached peer retains a
            // TLS socket after this fixture reports completion.
            timeout(Duration::from_secs(5), async {
                while let Some(frame) = peer.next().await {
                    if matches!(frame?, Frame::Close(_)) {
                        break;
                    }
                }
                peer.flush().await?;
                Ok::<_, anyhow::Error>(())
            })
            .await??;
            Ok(true)
        });
        Ok(Self { origin, task })
    }

    async fn finish(mut self) -> Result<bool> {
        timeout(Duration::from_secs(6), &mut self.task).await??
    }
}

impl Drop for WssServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn private_panel_ca_authenticates_wss_and_preserves_certificate_verification() -> Result<()> {
    let material = Material::new().await?;
    let identity = Identity {
        server_id: 7,
        signing_key: SigningKey::from_bytes(&[23; 32]),
    };
    let server = WssServer::start(
        &material,
        CertificateKind::Valid,
        identity.signing_key.verifying_key(),
    )
    .await?;
    let config = material.config(&server.origin);
    let (mut peer, ack) =
        timeout(Duration::from_secs(6), authenticate(&config, &identity)).await??;
    assert_eq!(ack.session_token, "TEST_ONLY_private_panel_session");
    assert_eq!(ack.session_expires_at, 3_610);
    peer.close(None).await?;
    assert!(
        server.finish().await?,
        "the real device signature must reach the trusted WSS panel"
    );

    for (kind, configured_ca) in [
        (CertificateKind::Valid, false),
        (CertificateKind::WrongName, true),
        (CertificateKind::Expired, true),
    ] {
        let server =
            WssServer::start(&material, kind, identity.signing_key.verifying_key()).await?;
        let mut config = material.config(&server.origin);
        if !configured_ca {
            config.panel_ca_file = None;
        }
        assert!(
            timeout(Duration::from_secs(6), authenticate(&config, &identity))
                .await?
                .is_err()
        );
        assert!(
            !server.finish().await?,
            "rejected TLS cannot transmit a device authentication signature"
        );
    }
    Ok(())
}
