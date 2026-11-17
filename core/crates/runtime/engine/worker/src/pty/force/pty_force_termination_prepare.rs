use super::{
    ForceOutput, ForceReady, PtyForceTermination, ReportedProgressReady, VersionedOutputFence,
};
use crate::process::KillPollReady;
use crate::worker::pty_output_progress::MasterAliasReleasePoll;
use crate::worker::pty_process_custody::PtyProcessCustody;
use anyhow::{Context, Result};

pub(crate) const KERNEL_EOF_PERMIT_MISSING_DETAIL: &str =
    "kernel EOF observation did not provide a freeze permit";

impl PtyForceTermination {
    pub(crate) fn prepare(
        &mut self,
        custody: &mut PtyProcessCustody,
        output: &mut ForceOutput<'_>,
    ) -> Result<ForceReady> {
        match custody {
            PtyProcessCustody::Tracking(tree) => {
                tree.require_force_transaction_start()?;
                self.output_fence = None;
                Ok(ForceReady::Tracking)
            }
            PtyProcessCustody::Freezing(tree) => self.prepare_freezing(tree, output),
            PtyProcessCustody::Frozen(tree) => self.prepare_frozen(tree, output),
            PtyProcessCustody::Killing(tree) => {
                let ready = tree.prepare_poll()?;
                if matches!(ready, KillPollReady::Refreeze) {
                    self.output_fence = None;
                }
                Ok(ForceReady::Killing(ready))
            }
            PtyProcessCustody::Survivor(tree) => tree.prepare_poll().map(ForceReady::Survivor),
            PtyProcessCustody::GonePending(completion) => completion
                .prepare_report(|authority| output.report_producers_gone(authority))
                .map(ForceReady::GonePending),
            PtyProcessCustody::GoneReported(completion) => {
                let progress = output.progress()?;
                ReportedProgressReady::prepare(completion, progress).map(ForceReady::GoneReported)
            }
            PtyProcessCustody::MasterClosePending(_) => {
                Ok(match output.release_master_aliases()? {
                    MasterAliasReleasePoll::Pending => ForceReady::Unchanged,
                    MasterAliasReleasePoll::Closed(proof) => ForceReady::MasterClosed(Some(proof)),
                    MasterAliasReleasePoll::Delivered => ForceReady::MasterClosed(None),
                })
            }
        }
    }

    pub(crate) fn prepare_freezing(
        &mut self,
        tree: &mut crate::process::FreezingProcessTree,
        output: &mut ForceOutput<'_>,
    ) -> Result<ForceReady> {
        let ready = tree.prepare_poll()?;
        let Some(version) = ready.kernel_eof_version().cloned() else {
            return Ok(ForceReady::Freezing(ready));
        };
        self.output_fence = None;
        let Some(mut authority) = output.acquire_kernel_eof_authority(&version)? else {
            return Ok(ForceReady::Freezing(ready));
        };
        if !output.validates_kernel_eof_authority(&authority, &version)? {
            return Ok(ForceReady::Freezing(ready));
        }
        let permit = authority
            .take_permit()
            .context(KERNEL_EOF_PERMIT_MISSING_DETAIL)?;
        let state = output.begin_fence()?;
        self.output_fence = Some(VersionedOutputFence {
            version,
            state,
            kernel_eof: Some(authority),
        });
        self.mark_progress();
        Ok(ForceReady::Freezing(ready.authorize_kernel_eof(permit)))
    }
}
