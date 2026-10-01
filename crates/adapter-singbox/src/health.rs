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
    match listener.transport {
        Transport::Tcp => {
            config.alpn_protocols = tls["alpn"]
                .as_array()
                .map(|protocols| {
                    protocols
                        .iter()
                        .filter_map(|value| value.as_str().map(|value| value.as_bytes().to_vec()))
                        .collect()
                })
                .unwrap_or_default();
            let socket = TcpStream::connect(listener.address).await?;
            TlsConnector::from(Arc::new(config))
                .connect(ServerName::try_from(name.to_owned())?, socket)
                .await?;
        }
        Transport::Udp => {
            config.alpn_protocols = vec![b"h3".to_vec()];
            let crypto = quinn::crypto::rustls::QuicClientConfig::try_from(config)?;
            let bind: SocketAddr = if listener.address.is_ipv6() {
                "[::1]:0"
            } else {
                "127.0.0.1:0"
            }
            .parse()?;
            let mut endpoint = quinn::Endpoint::client(bind)?;
            endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(crypto)));
            let connection = endpoint.connect(listener.address, name)?.await?;
            connection.close(0u32.into(), b"health check complete");
            endpoint.close(0u32.into(), b"health check complete");
        }
    }
    Ok(())
}
