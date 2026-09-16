//! Bound active HTTP connections, including idle clients and long-lived SSE.
use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{self, AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore},
};

pub(crate) struct BoundedListener {
    listener: TcpListener,
    slots: Arc<Semaphore>,
}

impl BoundedListener {
    pub(crate) fn new(listener: TcpListener, capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            listener,
            slots: Arc::new(Semaphore::new(capacity)),
        }
    }
}

pub(crate) struct Connection {
    stream: TcpStream,
    _slot: OwnedSemaphorePermit,
}

impl axum::serve::Listener for BoundedListener {
    type Io = Connection;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Connection, SocketAddr) {
        let slot = self
            .slots
            .clone()
            .acquire_owned()
            .await
            .expect("Connection slots never closed");
        let (stream, address) = axum::serve::Listener::accept(&mut self.listener).await;
        (
            Connection {
                stream,
                _slot: slot,
            },
            address,
        )
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}

impl AsyncRead for Connection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buffer)
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buffer)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::serve::Listener;
    use std::time::Duration;

    #[tokio::test]
    async fn idle_connections_are_bounded_and_dropping_one_releases_capacity() {
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = tcp.local_addr().unwrap();
        let mut listener = BoundedListener::new(tcp, 1);
        let _first = TcpStream::connect(address).await.unwrap();
        let (accepted, _) = listener.accept().await;
        let _second = TcpStream::connect(address).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), listener.accept())
                .await
                .is_err()
        );
        drop(accepted);
        let _ = tokio::time::timeout(Duration::from_secs(1), listener.accept())
            .await
            .unwrap();
    }
}
