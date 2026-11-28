use super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use crate::process::{PtyHostReleaseVerifier, PtyHostReleasedAck};
use crate::pty_custody::ExactCustodyReleasedAck;
use anyhow::{Context, Result};
use std::sync::{Arc, mpsc};
use tokio::sync::mpsc as async_mpsc;

const RELEASE_COMPLETION_CAPACITY: usize = 1;
const RELEASE_COMMAND_CLOSED_CONTEXT: &str = "PTY custody release command channel is closed";

pub(crate) type WorkerCustodyReleaseResult =
    std::result::Result<PtyHostReleasedAck, WorkerCustodyReleaseFailure>;

#[derive(Clone)]
pub(crate) struct WorkerCustodyReleaseHandle {
    identity: ReadyWorkerCustodyIdentity,
    commands: async_mpsc::UnboundedSender<WorkerCustodyReleaseCommand>,
}

pub(crate) struct WorkerCustodyReleaseCommand {
    pub(crate) verifier: PtyHostReleaseVerifier,
    completion: mpsc::SyncSender<WorkerCustodyReleaseResult>,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkerCustodyReleaseFailure {
    detail: Arc<str>,
}

pub(crate) fn channel(
    identity: ReadyWorkerCustodyIdentity,
) -> (
    WorkerCustodyReleaseHandle,
    async_mpsc::UnboundedReceiver<WorkerCustodyReleaseCommand>,
) {
    let (commands, receiver) = async_mpsc::unbounded_channel();
    (WorkerCustodyReleaseHandle { identity, commands }, receiver)
}

impl WorkerCustodyReleaseHandle {
    pub(crate) fn identity(&self) -> ReadyWorkerCustodyIdentity {
        self.identity
    }

    pub(crate) fn begin(
        &self,
        verifier: PtyHostReleaseVerifier,
    ) -> Result<mpsc::Receiver<WorkerCustodyReleaseResult>> {
        let (completion, receiver) = mpsc::sync_channel(RELEASE_COMPLETION_CAPACITY);
        self.commands
            .send(WorkerCustodyReleaseCommand {
                verifier,
                completion,
            })
            .context(RELEASE_COMMAND_CLOSED_CONTEXT)?;
        Ok(receiver)
    }
}

impl WorkerCustodyReleaseCommand {
    pub(crate) fn release_control(
        &self,
        offer: &crate::pty_custody::CustodyOffer,
    ) -> Result<crate::pty_custody::CustodyControl> {
        self.verifier.release_control(offer)
    }

    pub(crate) fn acknowledge(self, exact: ExactCustodyReleasedAck) {
        let acknowledgement = self.verifier.acknowledge(exact);
        let _ = self.completion.send(Ok(acknowledgement));
    }

    pub(crate) fn fail(self, error: &anyhow::Error) {
        let _ = self
            .completion
            .send(Err(WorkerCustodyReleaseFailure::from_error(error)));
    }
}

impl WorkerCustodyReleaseFailure {
    pub(crate) fn from_error(error: &anyhow::Error) -> Self {
        Self {
            detail: format!("{error:#}").into(),
        }
    }

    pub(crate) fn detail(&self) -> &str {
        &self.detail
    }
}
