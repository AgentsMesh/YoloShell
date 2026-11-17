use super::constants::{DELIVERY_RETRY_DELAY, DELIVERY_TIMEOUT, errors};
use super::{CodexNativeAckCode, CodexNativeRequest, CodexNativeSpool};
use anyhow::{Context, Result, ensure};
use std::path::Path;
use tokio::time::Instant;

const SERVER_UID: &str = "Codex native callback server uid does not match the client";
const NATIVE_REJECTED: &str = "Codex native callback was rejected";

pub fn run_native(paths: crate::paths::Paths) -> Result<()> {
    let environment = super::native_environment::NativeEnvironment::capture(&paths)?;
    let request = environment.request(super::read_stdin()?)?;
    let spool =
        CodexNativeSpool::for_scope(&paths, request.terminal_id, request.runtime_epoch)?;
    let occurrence = spool.persist(&request)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    runtime.block_on(deliver(
        &environment.socket,
        &request,
        Instant::now() + DELIVERY_TIMEOUT,
    ))?;
    spool.remove(&occurrence)
}

pub(super) async fn deliver(
    socket: &Path,
    request: &CodexNativeRequest,
    deadline: Instant,
) -> Result<()> {
    loop {
        let result = tokio::time::timeout_at(deadline, send_once(socket, request))
            .await
            .context(errors::DELIVERY_DEFERRED)?;
        let error = match result {
            Ok(ack) => {
                ack.validate_for(request)?;
                if ack.accepted {
                    return Ok(());
                }
                ensure!(
                    matches!(
                        ack.code,
                        CodexNativeAckCode::NotReady | CodexNativeAckCode::ProcessUnavailable
                    ),
                    NATIVE_REJECTED
                );
                anyhow::anyhow!(errors::DELIVERY_DEFERRED)
            }
            Err(error) => error,
        };
        if deadline.saturating_duration_since(Instant::now()) <= DELIVERY_RETRY_DELAY {
            return Err(error);
        }
        tokio::time::timeout_at(deadline, tokio::time::sleep(DELIVERY_RETRY_DELAY))
            .await
            .context(errors::DELIVERY_DEFERRED)?;
    }
}

async fn send_once(socket: &Path, request: &CodexNativeRequest) -> Result<super::CodexNativeAck> {
    let mut stream = crate::local_unix::LocalUnixStream::connect(socket).await?;
    let peer = stream.peer_cred()?;
    // SAFETY: geteuid only reads the current process credentials.
    ensure!(peer.uid() == unsafe { libc::geteuid() }, SERVER_UID);
    super::write_native_request(&mut stream, request).await?;
    super::read_native_ack(&mut stream).await
}
