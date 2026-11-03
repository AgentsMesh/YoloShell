use super::{
    super::federation_support::{POLL, deadline, listener},
    constants::*,
};
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
    task::{JoinHandle, JoinSet},
    time::timeout_at,
};

pub struct RouteProxy {
    pub address: SocketAddr,
    enabled: watch::Sender<bool>,
    stop: watch::Sender<bool>,
    active: Arc<AtomicUsize>,
    task: Option<JoinHandle<()>>,
}

impl RouteProxy {
    pub async fn start(origin: SocketAddr) -> Self {
        let listener = listener().await;
        let address = listener.local_addr().unwrap();
        let (enabled, current) = watch::channel(true);
        let (stop, stopped) = watch::channel(false);
        let active = Arc::new(AtomicUsize::new(usize::default()));
        let task = tokio::spawn(accept(listener, origin, current, stopped, active.clone()));
        Self {
            address,
            enabled,
            stop,
            active,
            task: Some(task),
        }
    }

    pub async fn cut(&self) {
        assert!(self.active.load(Ordering::Acquire) > usize::default());
        self.enabled.send_replace(false);
        timeout_at(deadline(), async {
            while self.active.load(Ordering::Acquire) != usize::default() {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
    }

    pub fn restore(&self) {
        self.enabled.send_replace(true);
    }

    pub async fn stop(mut self) {
        self.stop.send_replace(true);
        let mut task = self.task.take().unwrap();
        if let Ok(result) = timeout_at(deadline(), &mut task).await {
            result.unwrap();
        } else {
            task.abort();
            let _ = task.await;
            panic!("multipath proxy did not stop");
        }
        assert_eq!(self.active.load(Ordering::Acquire), usize::default());
    }
}

impl Drop for RouteProxy {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn accept(
    listener: TcpListener,
    origin: SocketAddr,
    enabled: watch::Receiver<bool>,
    mut stopped: watch::Receiver<bool>,
    active: Arc<AtomicUsize>,
) {
    let mut streams = JoinSet::new();
    loop {
        tokio::select! {
            _ = stopped.changed() => break,
            _ = streams.join_next(), if !streams.is_empty() => {},
            accepted = listener.accept() => {
                let (client, _) = accepted.unwrap();
                if !*enabled.borrow() || streams.len() >= MAX_PROXY_STREAMS { continue; }
                streams.spawn(forward(client, origin, enabled.clone(), stopped.clone(), active.clone()));
            }
        }
    }
    streams.abort_all();
    while streams.join_next().await.is_some() {}
}

async fn forward(
    mut client: TcpStream,
    origin: SocketAddr,
    mut enabled: watch::Receiver<bool>,
    mut stopped: watch::Receiver<bool>,
    active: Arc<AtomicUsize>,
) {
    if !*enabled.borrow_and_update() || *stopped.borrow() {
        return;
    }
    let connected = tokio::select! {
        _ = stopped.changed() => return,
        _ = enabled.changed() => return,
        result = timeout_at(deadline(), TcpStream::connect(origin)) => result,
    };
    let Ok(Ok(mut server)) = connected else {
        return;
    };
    active.fetch_add(ACTIVE_STREAM_STEP, Ordering::AcqRel);
    let _count = ActiveStream(active);
    tokio::select! {
        _ = stopped.changed() => {},
        _ = enabled.changed() => {},
        _ = tokio::io::copy_bidirectional(&mut client, &mut server) => {},
    }
}

struct ActiveStream(Arc<AtomicUsize>);
impl Drop for ActiveStream {
    fn drop(&mut self) {
        self.0.fetch_sub(ACTIVE_STREAM_STEP, Ordering::AcqRel);
    }
}
