use super::WorkerState;
use super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use super::secondary_custody_release::{
    WorkerCustodyReleaseCommand, WorkerCustodyReleaseHandle, channel,
};
use crate::pty_custody::{
    CustodyControl, CustodyControlAck, CustodyDurableRecord, CustodyGuardianBinding,
    CustodyHolderIdentity, CustodyOffer, CustodyUnixStream, CustodyWorkerActivated,
    CustodyWorkerReadyDisposition, ExactCustodyReleasedAck,
};
use anyhow::{Context, Result};
use std::sync::Arc;

const RELEASE_COMMAND_GONE_CONTEXT: &str = "PTY custody release command source disappeared";
const RELEASE_ACK_MISMATCH_DETAIL: &str =
    "Host PTY custody Released acknowledgement does not match its exact Release";
const SUPERSEDE_MISMATCH_DETAIL: &str =
    "Host PTY custody Supersede does not match the active predecessor lease";
const ACTIVATION_ALREADY_CONSUMED_CONTEXT: &str =
    "Worker PTY custody activation was already consumed";
const ACTIVATION_REQUIRED_DETAIL: &str = "Worker cannot hold PTY custody before exact activation";
#[path = "secondary_custody_pre_activation.rs"]
mod pre_activation;

pub(crate) struct WorkerCustodyLease {
    stream: CustodyUnixStream,
    identity: ReadyWorkerCustodyIdentity,
    offer: CustodyOffer,
    holder: CustodyHolderIdentity,
    activation: Option<CustodyWorkerActivated>,
    release_handle: WorkerCustodyReleaseHandle,
    release_commands: tokio::sync::mpsc::UnboundedReceiver<WorkerCustodyReleaseCommand>,
}

impl WorkerCustodyLease {
    pub(crate) fn new(
        stream: CustodyUnixStream,
        offer: CustodyOffer,
        holder: CustodyHolderIdentity,
        controller_generation: u64,
        guardian: Option<CustodyGuardianBinding>,
    ) -> Result<Self> {
        let identity =
            ReadyWorkerCustodyIdentity::new(&offer, holder, controller_generation, guardian);
        let (release_handle, release_commands) = channel(identity);
        let activation = CustodyWorkerActivated::new(&offer, controller_generation)?;
        Ok(Self {
            stream,
            identity,
            offer,
            holder,
            activation: Some(activation),
            release_handle,
            release_commands,
        })
    }

    pub(crate) fn identity(&self) -> ReadyWorkerCustodyIdentity {
        self.identity
    }

    pub(crate) fn release_handle(&self) -> WorkerCustodyReleaseHandle {
        self.release_handle.clone()
    }

    pub(crate) fn holder(&self) -> CustodyHolderIdentity {
        self.holder
    }

    pub(crate) fn profile(&self) -> crate::pty_custody::CustodyProfile {
        self.offer.profile
    }

    pub(crate) fn matches_durable_candidate(&self, durable: &CustodyDurableRecord) -> bool {
        durable.matches_candidate(&self.offer, self.identity.durable_lease())
    }

    pub(crate) fn is_released_successor(&self) -> bool {
        self.offer.descriptor_set == crate::pty_custody::CustodyDescriptorSet::ReleasedSuccessor
    }

    pub(crate) async fn activate(mut self) -> Result<Self> {
        let activation = self
            .activation
            .take()
            .context(ACTIVATION_ALREADY_CONSUMED_CONTEXT)?;
        self.stream
            .write(&CustodyWorkerReadyDisposition::activated(activation))
            .await?;
        Ok(self)
    }

    #[cfg(test)]
    pub(crate) fn offer_for_test(&self) -> CustodyOffer {
        self.offer.clone()
    }

    pub(crate) async fn hold(mut self, state: Arc<WorkerState>) -> Result<()> {
        anyhow::ensure!(self.activation.is_none(), ACTIVATION_REQUIRED_DETAIL);
        let result = self.hold_inner().await;
        state.mark_custody_unavailable(self.identity).await;
        result
    }

    async fn hold_inner(&mut self) -> Result<()> {
        tokio::select! {
            control = self.stream.read::<CustodyControl>() => {
                self.acknowledge_supersede(control?).await
            }
            command = self.release_commands.recv() => {
                self.release_to_host(command.context(RELEASE_COMMAND_GONE_CONTEXT)?).await
            }
        }
    }

    async fn acknowledge_supersede(&mut self, control: CustodyControl) -> Result<()> {
        let acknowledgement = CustodyControlAck::superseded(&control)
            .filter(|_| {
                matches!(
                    &control,
                    CustodyControl::Supersede { lease_id, offer_id, .. }
                        if *lease_id == self.identity.lease_id()
                            && *offer_id == self.identity.offer_id()
                )
            })
            .context(SUPERSEDE_MISMATCH_DETAIL)?;
        self.stream.write(&acknowledgement).await
    }

    async fn release_to_host(&mut self, command: WorkerCustodyReleaseCommand) -> Result<()> {
        let result = self.exchange_release(&command).await;
        match result {
            Ok(acknowledgement) => {
                command.acknowledge(acknowledgement);
                Ok(())
            }
            Err(error) => {
                command.fail(&error);
                Err(error)
            }
        }
    }

    async fn exchange_release(
        &mut self,
        command: &WorkerCustodyReleaseCommand,
    ) -> Result<ExactCustodyReleasedAck> {
        let control = command.release_control(&self.offer)?;
        self.stream.write(&control).await?;
        let acknowledgement: CustodyControlAck = self.stream.read().await?;
        acknowledgement
            .into_exact_released(&control)
            .context(RELEASE_ACK_MISMATCH_DETAIL)
    }

    #[cfg(test)]
    pub(crate) fn from_ready_for_test(
        stream: CustodyUnixStream,
        identity: ReadyWorkerCustodyIdentity,
        runtime: crate::pty_custody::CustodyRuntimeIdentity,
    ) -> Self {
        let (release_handle, release_commands) = channel(identity);
        let holder = identity.holder();
        let offer = CustodyOffer {
            protocol_version: crate::pty_custody::PROTOCOL_VERSION,
            lease_id: identity.lease_id(),
            offer_id: identity.offer_id(),
            runtime,
            #[cfg(target_os = "macos")]
            descriptor_set: crate::pty_custody::CustodyDescriptorSet::macos_process_capability(
                LEASE_TEST_READER_HANDLE,
                LEASE_TEST_WRITER_HANDLE,
            ),
            #[cfg(target_os = "linux")]
            descriptor_set: crate::pty_custody::CustodyDescriptorSet::LinuxContainment,
            profile: crate::pty_custody::CustodyProfile::ActiveRunning,
            recovery_identity_sha256: identity.recovery_identity_sha256(),
        };
        Self {
            stream,
            identity,
            offer,
            holder,
            activation: None,
            release_handle,
            release_commands,
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
const LEASE_TEST_READER_HANDLE: u64 = 70_001;
#[cfg(all(test, target_os = "macos"))]
const LEASE_TEST_WRITER_HANDLE: u64 = 70_002;
