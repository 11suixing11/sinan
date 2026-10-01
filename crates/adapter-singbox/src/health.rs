use crate::native::{Listener, Transport};
use anyhow::{Context, Result};
use sinan_adapter_sdk::Prepared;
use std::{io::ErrorKind, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpStream, UdpSocket},
    time::timeout,
};
use tokio_rustls::{
    TlsConnector,
    rustls::{
        self,
        pki_types::{CertificateDer, ServerName, pem::PemObject},
    },
};

pub(crate) fn budget(target: &Prepared) -> Duration {
    let acme = target
        .spec
        .files
        .get("config.json")
        .and_then(|config| serde_json::from_str::<serde_json::Value>(config).ok())
        .is_some_and(|config| {
            config["certificate_providers"]
                .as_array()
                .is_some_and(|providers| {
                    providers.iter().any(|provider| provider["type"] == "acme")
                })
        });
    if acme {
        Duration::from_secs(240)
    } else {
        crate::HEALTH_TIMEOUT
    }
}

pub(crate) async fn probe(listener: &Listener) -> bool {
    matches!(
        timeout(Duration::from_secs(2), probe_inner(listener)).await,
        Ok(Ok(()))
    )
}

async fn probe_inner(listener: &Listener) -> Result<()> {
    let Some(tls) = &listener.tls else {
        match listener.transport {
            Transport::Tcp => {
                TcpStream::connect(listener.address).await?;
            }
            Transport::Udp => {
                // UDP has no connect handshake. Service state and statistics are checked separately.
                match UdpSocket::bind(listener.address).await {
                    Err(error) if error.kind() == ErrorKind::AddrInUse => {}
                    _ => anyhow::bail!("UDP listener is not bound"),
                }
            }
        }
        return Ok(());
    };
    let name = tls["server_name"].as_str().context("missing TLS name")?;
    let mut roots = rustls::RootCertStore::empty();
    if let Some(certificate) = tls.get("certificate") {
        let pem = if let Some(value) = certificate.as_str() {
            value.to_owned()
        } else {
            certificate
                .as_array()
                .context("invalid certificate chain")?
                .iter()
                .map(|line| line.as_str().context("invalid certificate line"))
                .collect::<Result<Vec<_>>>()?
                .join("\n")
        };
        for certificate in CertificateDer::pem_slice_iter(pem.as_bytes()) {
            roots.add(certificate?)?;
        }
        anyhow::ensure!(!roots.is_empty(), "empty certificate chain");
    } else {
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = tls["alpn"]
        .as_array()
        .map(|protocols| {
            protocols
                .iter()
                .filter_map(|value| value.as_str().map(|value| value.as_bytes().to_vec()))
                .collect()
        })
        .unwrap_or_else(|| {
            if listener.transport == Transport::Udp {
                vec![b"h3".to_vec()]
            } else {
                vec![]
            }
        });
    match listener.transport {
        Transport::Tcp => {
            let socket = TcpStream::connect(listener.address).await?;
            TlsConnector::from(Arc::new(config))
                .connect(ServerName::try_from(name.to_owned())?, socket)
                .await?;
        }
        Transport::Udp => {
            let bridge = match &listener.obfuscation {
                Some(password) => {
                    Some(crate::obfuscation::Bridge::new(listener.address, password.clone()).await?)
                }
                None => None,
            };
            let destination = bridge
                .as_ref()
                .map_or(listener.address, |bridge| bridge.address);
            let crypto = quinn::crypto::rustls::QuicClientConfig::try_from(config)?;
            let bind: SocketAddr = if destination.is_ipv6() {
                "[::]:0"
            } else {
                "0.0.0.0:0"
            }
            .parse()?;
            let mut endpoint = quinn::Endpoint::client(bind)?;
            endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(crypto)));
            let connection = endpoint.connect(destination, name)?.await?;
            connection.close(0u32.into(), b"health check complete");
            endpoint.close(0u32.into(), b"health check complete");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    #[ignore = "requires OpenSSL and official sing-box 1.14.2 in SINAN_TEST_UPSTREAM; runs only on ephemeral loopback ports"]
    async fn obfuscated_quic_health_checks_certificate_and_custom_alpn() -> Result<()> {
        let binary = std::env::var("SINAN_TEST_UPSTREAM")?;
        let directory = std::env::temp_dir().join(format!("sinan-health-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory)?;
        let certificate = directory.join("certificate.pem");
        let key = directory.join("key.pem");
        let output = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=proxy.example.com",
                "-addext",
                "subjectAltName=DNS:proxy.example.com",
                "-addext",
                "basicConstraints=critical,CA:FALSE",
                "-keyout",
            ])
            .arg(&key)
            .arg("-out")
            .arg(&certificate)
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "test certificate generation failed"
        );
        let certificate = std::fs::read_to_string(certificate)?;
        let key = std::fs::read_to_string(key)?;
        let reserved = UdpSocket::bind("127.0.0.1:0").await?;
        let address = reserved.local_addr()?;
        drop(reserved);
        let password = "synthetic-test-obfuscation";
        let tls = json!({"enabled":true,"server_name":"proxy.example.com","certificate":certificate,"key":key,"alpn":["sinan-test"]});
        let path = directory.join("config.json");
        std::fs::write(&path, json!({"log":{"disabled":true},"inbounds":[{
            "type":"hysteria2","listen":"127.0.0.1","listen_port":address.port(),"users":[{"name":"test","password":"synthetic-test-credential"}],
            "obfs":{"type":"salamander","password":password},"tls":tls
        }],"outbounds":[{"type":"direct"}]}).to_string())?;
        let mut child = tokio::process::Command::new(binary)
            .arg("run")
            .arg("-c")
            .arg(path)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let mut listener = Listener {
            address,
            transport: Transport::Udp,
            tls: Some(tls),
            obfuscation: Some(password.into()),
        };
        let result = async {
            timeout(Duration::from_secs(10), async {
                loop {
                    if probe(&listener).await {
                        break;
                    }
                    anyhow::ensure!(child.try_wait()?.is_none(), "test runtime exited");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Ok::<_, anyhow::Error>(())
            })
            .await??;
            listener.obfuscation = Some("wrong-synthetic-password".into());
            anyhow::ensure!(
                !probe(&listener).await,
                "incorrect obfuscation was accepted"
            );
            listener.obfuscation = Some(password.into());
            listener.tls.as_mut().unwrap()["server_name"] = json!("wrong.example.com");
            anyhow::ensure!(
                !probe(&listener).await,
                "incorrect certificate name was accepted"
            );
            Ok::<_, anyhow::Error>(())
        }
        .await;
        child.kill().await?;
        child.wait().await?;
        std::fs::remove_dir_all(directory)?;
        result
    }
}
