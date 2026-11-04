use super::CodexHookContext;
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{CodexNativeRequest, CodexNativeSpool};
use crate::worker::codex_hook::router::CodexRouteLease;
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

#[path = "native_transaction_install.rs"]
mod install;
#[path = "native_namespace.rs"]
mod namespace;

const NATIVE_DURABLE_REQUEST: &str = "native Codex callback has no exact durable request";
const NATIVE_NAMESPACE_CAPTURE: &str = "capture native Codex callback namespace";
const NATIVE_SPOOL_OPEN: &str = "open native Codex callback spool";
const NATIVE_SPOOL_READ: &str = "read native Codex callback occurrence";
const NATIVE_ADMISSION_INSTALL: &str = "install native Codex callback admission";

#[derive(Debug, thiserror::Error)]
#[error("native Codex callback process or evidence preflight is unavailable")]
pub(in crate::worker::codex_hook) struct NativePreflightFailure;

pub(in crate::worker::codex_hook) struct NativeCodexHandoff {
    pub(in crate::worker::codex_hook) lease: CodexRouteLease,
    pub(in crate::worker::codex_hook) event: HookEnvelope,
    pub(in crate::worker::codex_hook) first_event: bool,
}

impl CodexHookContext {
    pub(in crate::worker::codex_hook) async fn admit_native_callback(
        &self,
    ) -> Result<tokio::sync::MutexGuard<'_, ()>> {
        tokio::time::timeout(super::super::constants::CODEX_REQUEST_TIMEOUT, async {
            let guard = self.native_callbacks.lock().await;
            self.settlements.wait_idle().await;
            guard
        })
        .await
        .context(super::super::constants::CODEX_REQUEST_TIMEOUT_ERROR)
    }

    pub(in crate::worker::codex_hook) async fn execute_native(
        &self,
        peer: crate::process::ProcessIdentity,
        request: CodexNativeRequest,
    ) -> Result<NativeCodexHandoff> {
        let admitted = super::super::native_admission::admit(
            peer,
            self.shell,
            &request,
            &self.policy,
            Arc::clone(&self.process_source),
            self.settlements.clone(),
            &self.generation,
        )
        .await
        .context(NativePreflightFailure)?;
        let transaction = self.clone();
        let settlement = self.settlements.admit();
        tokio::task::spawn_blocking(move || {
            let _settlement = settlement;
            let namespace = namespace::acquire(&transaction).context(NATIVE_NAMESPACE_CAPTURE)?;
            let spool = CodexNativeSpool::for_scope(
                &transaction.paths,
                request.terminal_id,
                request.runtime_epoch,
            )
            .context(NATIVE_SPOOL_OPEN)?;
            let occurrence = spool
                .read(request.event_id)
                .context(NATIVE_SPOOL_READ)?
                .context(NATIVE_DURABLE_REQUEST)?;
            ensure!(occurrence.request() == &request, NATIVE_DURABLE_REQUEST);
            install::install(&transaction, peer, request, admitted, || {
                namespace.verify()?;
                spool.verify(&occurrence).context(NATIVE_SPOOL_READ)
            })
            .context(NATIVE_ADMISSION_INSTALL)
        })
        .await?
    }
}
