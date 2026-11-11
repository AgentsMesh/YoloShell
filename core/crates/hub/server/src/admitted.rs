use anyhow::{Result, ensure};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use yolo_hub_client_access_dispatch::{ScopedDispatcher, while_authorized};
use yolo_hub_client_access_registry::ClientAdmission;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_state::HubState;

/// The SSH transport supplies an admission only after proving client-key
/// possession. An outer socket's credentials never grant local management.
pub async fn serve_admitted<S: AsyncRead + AsyncWrite + Send + Unpin>(
    stream: S,
    dispatcher: Arc<dyn Dispatcher>,
    state: Arc<HubState>,
    admission: ClientAdmission,
) -> Result<()> {
    serve_admitted_audited(stream, dispatcher, state, admission, None).await
}

pub async fn serve_admitted_audited<S: AsyncRead + AsyncWrite + Send + Unpin>(
    stream: S,
    dispatcher: Arc<dyn Dispatcher>,
    state: Arc<HubState>,
    admission: ClientAdmission,
    audit: Option<Arc<yolo_hub_audit::AuditLog>>,
) -> Result<()> {
    ensure!(
        state.hub_id() == dispatcher.hub_id(),
        crate::constants::IDENTITY_MISMATCH
    );
    let scoped: Arc<dyn Dispatcher> = Arc::new(ScopedDispatcher::new(dispatcher, admission.clone())?);
    let scoped = match audit {
        Some(log) => Arc::new(yolo_hub_audit::AuditedDispatcher::new(
            scoped,
            log,
            &admission.grant().identity.client_id.to_string(),
        )),
        None => scoped,
    };
    // Includes blocked reads/writes and attachment heartbeats. Quiet streams
    // must also close on revocation or expiry, without waiting for another RPC.
    while_authorized(&admission, async {
        Ok(crate::connection::serve_stream(stream, scoped, state).await)
    })
    .await??;
    Ok(())
}
