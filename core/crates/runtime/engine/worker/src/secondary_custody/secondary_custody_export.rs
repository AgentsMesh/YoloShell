use super::lock_recover;
use super::secondary_custody_source::{PlatformCustodyExport, WorkerSecondaryCustodySource};
use crate::pty_custody::{CustodyHolderIdentity, CustodyUnixStream};
use anyhow::Result;
use std::sync::{Arc, Mutex};

const EXPORT_FENCED_DETAIL: &str =
    "PTY successor descriptor export was fenced by runtime alias closure";
const EXPORT_COUNT_OVERFLOW_DETAIL: &str = "PTY successor export lease count overflowed";
const EXPORT_COUNT_UNDERFLOW_DETAIL: &str = "PTY successor export lease count underflowed";

#[derive(Default)]
pub(crate) struct SecondaryCustodyExportState {
    inner: Mutex<ExportState>,
    changed: tokio::sync::Notify,
}

#[derive(Default)]
struct ExportState {
    fenced: bool,
    active: usize,
    unresolved: Vec<CustodyHolderIdentity>,
}

enum ExportDisposition {
    Local,
    Exported(CustodyHolderIdentity),
    Resolved,
}

pub(crate) struct SuccessorCustodyExportLease {
    source: Arc<WorkerSecondaryCustodySource>,
    platform: Option<Arc<PlatformCustodyExport>>,
    disposition: ExportDisposition,
}

impl WorkerSecondaryCustodySource {
    pub(crate) fn lease_successor_export(self: &Arc<Self>) -> Result<SuccessorCustodyExportLease> {
        let mut state = lock_recover(&self.successor_exports.inner);
        anyhow::ensure!(!state.fenced, EXPORT_FENCED_DETAIL);
        let platform = self.platform()?;
        state.active = state
            .active
            .checked_add(1)
            .expect(EXPORT_COUNT_OVERFLOW_DETAIL);
        Ok(SuccessorCustodyExportLease {
            source: Arc::clone(self),
            platform: Some(platform),
            disposition: ExportDisposition::Local,
        })
    }

    pub(crate) fn fence_successor_exports(&self) -> bool {
        let mut state = lock_recover(&self.successor_exports.inner);
        state.fenced = true;
        state
            .unresolved
            .retain(|holder| holder.prove_gone().is_none());
        let settled = state.active == 0 && state.unresolved.is_empty();
        drop(state);
        self.successor_exports.changed.notify_waiters();
        settled
    }

    pub(crate) async fn wait_for_successor_export_progress(&self) {
        let changed = self.successor_exports.changed.notified();
        tokio::pin!(changed);
        if self.successor_exports_settled() {
            return;
        }
        tokio::select! {
            () = &mut changed => {}
            () = tokio::time::sleep(crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL) => {}
        }
    }

    fn successor_exports_settled(&self) -> bool {
        let mut state = lock_recover(&self.successor_exports.inner);
        state
            .unresolved
            .retain(|holder| holder.prove_gone().is_none());
        state.active == 0 && state.unresolved.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn successor_export_counts_for_test(&self) -> (usize, usize) {
        let state = lock_recover(&self.successor_exports.inner);
        (state.active, state.unresolved.len())
    }
}

impl SuccessorCustodyExportLease {
    pub(crate) fn source(&self) -> &WorkerSecondaryCustodySource {
        &self.source
    }

    pub(crate) async fn send_descriptors(
        &mut self,
        stream: &CustodyUnixStream,
        holder: CustodyHolderIdentity,
    ) -> Result<()> {
        self.source.prepare_descriptor_send(holder)?;
        let platform = self
            .platform
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(EXPORT_FENCED_DETAIL))?;
        stream.send_rights(&platform.as_descriptors()).await?;
        self.disposition = ExportDisposition::Exported(holder);
        Ok(())
    }

    pub(crate) fn closure_requested(&self) -> bool {
        lock_recover(&self.source.successor_exports.inner).fenced
    }

    pub(crate) async fn wait_for_closure_request(&self) {
        loop {
            let changed = self.source.successor_exports.changed.notified();
            if self.closure_requested() {
                return;
            }
            changed.await;
        }
    }

    pub(crate) fn resolve(&mut self) {
        drop(self.platform.take());
        self.disposition = ExportDisposition::Resolved;
    }
}

impl Drop for SuccessorCustodyExportLease {
    fn drop(&mut self) {
        drop(self.platform.take());
        let mut state = lock_recover(&self.source.successor_exports.inner);
        state.active = state
            .active
            .checked_sub(1)
            .expect(EXPORT_COUNT_UNDERFLOW_DETAIL);
        if let ExportDisposition::Exported(holder) = self.disposition {
            state.unresolved.push(holder);
        }
        drop(state);
        self.source.successor_exports.changed.notify_waiters();
    }
}
