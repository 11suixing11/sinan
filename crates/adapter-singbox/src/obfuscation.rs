use anyhow::Result;
use blake2::{Blake2b, Digest, digest::consts::U32};
use rand::RngCore;
use std::net::SocketAddr;
use tokio::{net::UdpSocket, task::JoinHandle};

// A short-lived loopback bridge lets the QUIC health handshake use the same
// Salamander framing as the listener, without a second runtime or persistence.
// Dropping the probe on success, error, cancellation or timeout stops the task.
pub(crate) struct Bridge {
    pub address: SocketAddr,
    task: JoinHandle<Result<()>>,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Bridge {
    pub async fn new(upstream: SocketAddr, password: String) -> Result<Self> {
        let local = UdpSocket::bind(if upstream.is_ipv6() {
            "[::1]:0"
        } else {
            "127.0.0.1:0"
        })
        .await?;
        let remote = UdpSocket::bind(if upstream.is_ipv6() {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        })
        .await?;
        remote.connect(upstream).await?;
        let address = local.local_addr()?;
        let task = tokio::spawn(async move {
            let mut outgoing = vec![0u8; 65_536];
            let mut incoming = vec![0u8; 65_536];
            let mut client = None;
            loop {
                tokio::select! {
                    packet = local.recv_from(&mut outgoing[8..]) => {
                        let (length, source) = packet?;
                        if client.is_some_and(|address| address != source) { continue; }
                        client = Some(source);
                        let mut salt = [0u8; 8];
                        rand::rngs::OsRng.fill_bytes(&mut salt);
                        outgoing[..8].copy_from_slice(&salt);
                        transform(&password, &salt, &mut outgoing[8..8+length]);
                        remote.send(&outgoing[..8+length]).await?;
                    }
                    packet = remote.recv(&mut incoming) => {
                        let length = packet?;
                        if length <= 8 { continue; }
                        let Some(client) = client else { continue };
                        let salt: [u8;8] = incoming[..8].try_into().expect("salt length");
                        transform(&password, &salt, &mut incoming[8..length]);
                        local.send_to(&incoming[8..length], client).await?;
                    }
                }
            }
        });
        Ok(Self { address, task })
    }
}

fn transform(password: &str, salt: &[u8; 8], payload: &mut [u8]) {
    let mut hash = Blake2b::<U32>::new();
    hash.update(password.as_bytes());
    hash.update(salt);
    let key = hash.finalize();
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= key[index % key.len()];
    }
}
