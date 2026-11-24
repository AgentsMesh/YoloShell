use super::BrokerConnectionPool;
use super::constants::*;
use crate::local_unix::LocalUnixStream;
use anyhow::Result;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::{
    io::AsyncReadExt,
    sync::{Mutex, Semaphore, oneshot, watch},
};

pub(super) enum Stage<T> {
    Complete(T),
    LocalEof,
}
pub(super) struct EstablishedRuntime {
    task: tokio::task::JoinHandle<Result<()>>,
    allowed: watch::Sender<bool>,
}
impl Drop for EstablishedRuntime {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl EstablishedRuntime {
    pub(super) async fn forward(mut self) -> Result<()> {
        self.allowed.send_replace(true);
        (&mut self.task).await?
    }
}
#[cfg(test)]
pub(super) async fn proxy(local: LocalUnixStream, pool: Arc<BrokerConnectionPool>) -> Result<()> {
    match setup(local, pool, None).await? {
        Some(runtime) => runtime.forward().await,
        None => Ok(()),
    }
}
#[cfg(test)]
pub(super) async fn proxy_admitted(
    local: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
    attachments: Arc<Semaphore>,
) -> Result<()> {
    match setup(local, pool, Some(attachments)).await? {
        Some(runtime) => runtime.forward().await,
        None => Ok(()),
    }
}
pub(super) async fn setup_admitted(
    local: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
    attachments: Arc<Semaphore>,
) -> Result<Option<EstablishedRuntime>> {
    setup(local, pool, Some(attachments)).await
}
async fn setup(
    local: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
    attachments: Option<Arc<Semaphore>>,
) -> Result<Option<EstablishedRuntime>> {
    let (ready, admitted) = oneshot::channel();
    let (allowed, receiver) = watch::channel(false);
    let seen = Arc::new(tokio::sync::Notify::new());
    let service = Arc::new(super::runtime_rpc::RuntimeProxy {
        pool,
        attachments,
        control: Mutex::new(None),
        admission: super::runtime_admission::RuntimeAdmission::new(ready, receiver, seen.clone()),
    });
    let task = tokio::spawn(async move {
        yolo_capnp_rpc::serve(local, yolo_capnp_rpc::Protocol::Endpoint, service).await?;
        Ok(())
    });
    let mut runtime = EstablishedRuntime { task, allowed };
    tokio::select! {
        _ = seen.notified() => {},
        _ = tokio::time::sleep(LOCAL_HANDSHAKE_TIMEOUT) => return Err(anyhow::anyhow!(LOCAL_PROTOCOL_TIMEOUT_MESSAGE)),
        result = &mut runtime.task => { result??; return Ok(None); },
    }
    tokio::select! {
        result = admitted => {
            if result.is_ok() { Ok(Some(runtime)) }
            else { (&mut runtime.task).await??; Ok(None) }
        },
        result = &mut runtime.task => { result??; Ok(None) },
    }
}

pub(super) async fn monitor<F, T>(
    local: &mut LocalUnixStream,
    prefix: &mut Vec<u8>,
    mut stage: Pin<&mut F>,
) -> Result<Stage<T>>
where
    F: Future<Output = T>,
{
    let mut buffer = [0_u8; RUNTIME_FORWARD_BUFFER_BYTES];
    loop {
        let remaining = MAX_RUNTIME_PENDING_INPUT_BYTES
            .saturating_add(1)
            .saturating_sub(prefix.len());
        let read_size = remaining.min(buffer.len());
        match local.try_read_kernel(&mut buffer[..read_size]) {
            Ok(bytes) if record_pending(prefix, &buffer, bytes)? => {
                return Ok(Stage::LocalEof);
            }
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        tokio::select! {
            biased;
            read = local.read(&mut buffer[..read_size]) => {
                let bytes = read?;
                if record_pending(prefix, &buffer, bytes)? {
                    return Ok(Stage::LocalEof);
                }
            }
            output = stage.as_mut() => return Ok(Stage::Complete(output)),
        }
    }
}

fn record_pending(prefix: &mut Vec<u8>, buffer: &[u8], bytes: usize) -> Result<bool> {
    if bytes == usize::default() {
        return Ok(true);
    }
    prefix.extend_from_slice(&buffer[..bytes]);
    if prefix.len() > MAX_RUNTIME_PENDING_INPUT_BYTES {
        return Err(anyhow::anyhow!(RUNTIME_PENDING_INPUT_LIMIT_MESSAGE));
    }
    Ok(false)
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/runtime/runtime_proxy_transport_tests.rs"]
mod transport_tests;
