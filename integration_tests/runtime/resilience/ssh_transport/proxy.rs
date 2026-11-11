use super::constants::{LOOPBACK_HOST, TRANSPORT_EVENT_TIMEOUT};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, oneshot, watch};

struct ActiveTransport {
    cut: oneshot::Sender<()>,
    closed: oneshot::Receiver<()>,
}

pub(super) struct TransportProxy {
    address: SocketAddr,
    active: Arc<Mutex<Option<ActiveTransport>>>,
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl TransportProxy {
    pub(super) async fn start(backend: SocketAddr) -> Self {
        let listener = TcpListener::bind((LOOPBACK_HOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let active = Arc::new(Mutex::new(None));
        let (shutdown, receiver) = watch::channel(false);
        let task = tokio::spawn(accept_connections(
            listener,
            backend,
            Arc::clone(&active),
            receiver,
        ));
        Self {
            address,
            active,
            shutdown,
            task,
        }
    }

    pub(super) fn port(&self) -> u16 {
        self.address.port()
    }

    pub(super) async fn cut_active_transport(&self) {
        let active = tokio::time::timeout(TRANSPORT_EVENT_TIMEOUT, async {
            loop {
                if let Some(active) = self.active.lock().await.take() {
                    return active;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("SSH transport was not established");
        let _ = active.cut.send(());
        tokio::time::timeout(TRANSPORT_EVENT_TIMEOUT, active.closed)
            .await
            .expect("SSH transport did not close")
            .expect("SSH transport close notifier disappeared");
    }
}

impl Drop for TransportProxy {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        self.task.abort();
    }
}

async fn accept_connections(
    listener: TcpListener,
    backend: SocketAddr,
    active: Arc<Mutex<Option<ActiveTransport>>>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((client, _)) = accepted else { return };
                let Ok(server) = TcpStream::connect(backend).await else { continue };
                let (cut, cut_request) = oneshot::channel();
                let (closed, closed_request) = oneshot::channel();
                *active.lock().await = Some(ActiveTransport {
                    cut,
                    closed: closed_request,
                });
                tokio::spawn(proxy_transport(client, server, cut_request, closed, shutdown.clone()));
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { return; }
            }
        }
    }
}

async fn proxy_transport(
    mut client: TcpStream,
    mut server: TcpStream,
    mut cut: oneshot::Receiver<()>,
    closed: oneshot::Sender<()>,
    mut shutdown: watch::Receiver<bool>,
) {
    tokio::select! {
        _ = tokio::io::copy_bidirectional(&mut client, &mut server) => {}
        _ = &mut cut => {}
        _ = shutdown.changed() => {}
    }
    drop((client, server));
    let _ = closed.send(());
}
