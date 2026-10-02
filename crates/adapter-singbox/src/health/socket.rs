use std::{
    future::Future,
    io::{self, IoSliceMut},
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tokio::{io::ReadBuf, net::UdpSocket, task::JoinHandle};

// Quinn keeps its abstract socket in connection drivers as well as the endpoint.
// These drivers may outlive an Endpoint handle. Only this object owns the actual
// descriptor; closing it also invalidates every poller without waiting for QUIC
// draining timers. No socket clone escapes the lock.
#[derive(Debug)]
pub(super) struct Socket {
    state: Mutex<SocketState>,
}

#[derive(Debug)]
struct SocketState {
    socket: Option<UdpSocket>,
    readers: Option<Waker>,
    writers: Option<Waker>,
}

impl Socket {
    pub fn bind(address: SocketAddr) -> io::Result<Arc<Self>> {
        let socket = std::net::UdpSocket::bind(address)?;
        socket.set_nonblocking(true)?;
        Ok(Arc::new(Self {
            state: Mutex::new(SocketState {
                socket: Some(UdpSocket::from_std(socket)?),
                readers: None,
                writers: None,
            }),
        }))
    }

    pub fn close(&self) {
        let (socket, reader, writer) = {
            let mut state = self.state.lock().expect("health socket lock");
            (
                state.socket.take(),
                state.readers.take(),
                state.writers.take(),
            )
        };
        drop(socket);
        if let Some(reader) = reader {
            reader.wake();
        }
        if let Some(writer) = writer {
            writer.wake();
        }
    }

    #[cfg(test)]
    pub fn is_closed(&self) -> bool {
        self.state
            .lock()
            .expect("health socket lock")
            .socket
            .is_none()
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "health probe socket closed")
}

impl quinn::AsyncUdpSocket for Socket {
    fn create_io_poller(self: Arc<Self>) -> Pin<Box<dyn quinn::UdpPoller>> {
        Box::pin(Poller(self))
    }

    fn try_send(&self, transmit: &quinn::udp::Transmit<'_>) -> io::Result<()> {
        // This handshake-only socket advertises one segment, with no GSO/GRO
        // or ECN. A multi-segment transmission is never silently truncated.
        if transmit
            .segment_size
            .is_some_and(|size| size < transmit.contents.len())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "segmented health datagram",
            ));
        }
        let state = self.state.lock().expect("health socket lock");
        let socket = state.socket.as_ref().ok_or_else(closed)?;
        let written = socket.try_send_to(transmit.contents, transmit.destination)?;
        if written != transmit.contents.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "partial health datagram",
            ));
        }
        Ok(())
    }

    fn poll_recv(
        &self,
        cx: &mut Context<'_>,
        buffers: &mut [IoSliceMut<'_>],
        metadata: &mut [quinn::udp::RecvMeta],
    ) -> Poll<io::Result<usize>> {
        let (Some(buffer), Some(metadata)) = (buffers.first_mut(), metadata.first_mut()) else {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "empty health receive buffer",
            )));
        };
        let mut state = self.state.lock().expect("health socket lock");
        state.readers = Some(cx.waker().clone());
        let Some(socket) = state.socket.as_ref() else {
            return Poll::Ready(Err(closed()));
        };
        let mut buffer = ReadBuf::new(buffer);
        match socket.poll_recv_from(cx, &mut buffer) {
            Poll::Ready(Ok(address)) => {
                let length = buffer.filled().len();
                *metadata = quinn::udp::RecvMeta {
                    addr: address,
                    len: length,
                    stride: length,
                    ecn: None,
                    dst_ip: None,
                };
                Poll::Ready(Ok(1))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.state
            .lock()
            .expect("health socket lock")
            .socket
            .as_ref()
            .ok_or_else(closed)?
            .local_addr()
    }
}

#[derive(Debug)]
struct Poller(Arc<Socket>);

impl quinn::UdpPoller for Poller {
    fn poll_writable(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut state = self.0.state.lock().expect("health socket lock");
        state.writers = Some(cx.waker().clone());
        match state.socket.as_ref() {
            Some(socket) => socket.poll_send_ready(cx),
            None => Poll::Ready(Err(closed())),
        }
    }
}

// Keep only the tasks spawned for this one endpoint, never unrelated Tokio work.
// Explicit cleanup aborts and joins these tasks; cancellation closes the socket
// and aborts the same set as a fallback. A closed runtime cannot spawn new work.
#[derive(Debug, Default)]
pub(super) struct Runtime {
    tasks: Mutex<Tasks>,
}

#[derive(Debug, Default)]
struct Tasks {
    closed: bool,
    handles: Vec<JoinHandle<()>>,
}

impl Runtime {
    pub fn abort(&self) -> Vec<JoinHandle<()>> {
        let mut tasks = self.tasks.lock().expect("health runtime lock");
        tasks.closed = true;
        for handle in &tasks.handles {
            handle.abort();
        }
        std::mem::take(&mut tasks.handles)
    }

    pub async fn finish(&self) -> anyhow::Result<()> {
        let mut failure = None;
        for task in self.abort() {
            if let Err(error) = task.await
                && !error.is_cancelled()
                && failure.is_none()
            {
                failure = Some(error);
            }
        }
        match failure {
            Some(error) => Err(anyhow::anyhow!("health driver failed: {error}")),
            None => Ok(()),
        }
    }
}

impl quinn::Runtime for Runtime {
    fn new_timer(&self, instant: std::time::Instant) -> Pin<Box<dyn quinn::AsyncTimer>> {
        quinn::Runtime::new_timer(&quinn::TokioRuntime, instant)
    }

    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let mut tasks = self.tasks.lock().expect("health runtime lock");
        if !tasks.closed {
            tasks.handles.push(tokio::spawn(future));
        }
    }

    fn wrap_udp_socket(
        &self,
        _socket: std::net::UdpSocket,
    ) -> io::Result<Arc<dyn quinn::AsyncUdpSocket>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "health runtime requires an owned socket",
        ))
    }

    fn now(&self) -> std::time::Instant {
        quinn::Runtime::now(&quinn::TokioRuntime)
    }
}

pub(super) struct Resources {
    pub endpoint: quinn::Endpoint,
    pub socket: Arc<Socket>,
    pub runtime: Arc<Runtime>,
}

impl Resources {
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"health check complete");
        self.socket.close();
    }

    pub async fn finish(&self) -> anyhow::Result<()> {
        self.close();
        self.runtime.finish().await
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        self.close();
        drop(self.runtime.abort());
    }
}
