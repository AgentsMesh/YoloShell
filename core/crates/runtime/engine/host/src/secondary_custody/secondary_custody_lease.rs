use super::*;
use std::sync::atomic::Ordering;

const MONITOR_CHANNEL_CAPACITY: usize = 1;
const INVALID_REQUEST_DETAIL: &str = "PTY custody lease request is invalid";
#[cfg(test)]
const MONITOR_UNAVAILABLE_DETAIL: &str = "PTY custody monitor is unavailable";

impl SecondaryCustodyLease {
    pub(super) fn new(
        host: &Host,
        request: &crate::pty_custody::CustodyRequest,
        offer: CustodyOffer,
        guardian: Option<crate::pty_custody::CustodyGuardianPreparation>,
        platform: PlatformSecondaryCustody,
        stream: CustodyUnixStream,
    ) -> Result<Self, WorkerRpcError> {
        if !request.is_well_formed() {
            return Err(WorkerRpcError::protocol(INVALID_REQUEST_DETAIL));
        }
        let (monitor_commands, monitor_receiver) =
            tokio::sync::mpsc::channel(MONITOR_CHANNEL_CAPACITY);
        let (state_events, _) = tokio::sync::watch::channel(SecondaryCustodyState::Preparing);
        Ok(Self {
            runtime: request.runtime.clone(),
            holder: request.holder(),
            controller_generation: request.controller_generation,
            host_boot_id: host.boot_id.clone(),
            request_drain: host.request_drain.clone(),
            offer,
            guardian,
            #[cfg(target_os = "macos")]
            durable_retirement: super::durable_retirement::SecondaryCustodyDurableRetirement::new(
                host,
            ),
            platform: Mutex::new(Some(platform)),
            stream: AsyncMutex::new(Some(stream)),
            monitor_commands,
            monitor_receiver: Mutex::new(Some(monitor_receiver)),
            registry: Mutex::new(None),
            state: Mutex::new(SecondaryCustodyState::Preparing),
            state_events,
            wire_ready: AtomicBool::new(false),
            available: AtomicBool::new(false),
            settlement_active: AtomicBool::new(false),
            #[cfg(test)]
            worker_identity_probe: Mutex::new(None),
        })
    }

    pub(super) fn matches(
        &self,
        record: &RuntimeRecord,
        holder: CustodyHolderIdentity,
        generation: u64,
    ) -> bool {
        self.runtime.matches_record(record)
            && self.holder == holder
            && self.controller_generation == generation
    }

    pub(super) fn is_ready(&self) -> bool {
        self.admission_snapshot().1
    }

    pub(super) fn profile(&self) -> crate::pty_custody::CustodyProfile {
        self.offer.profile
    }

    pub(super) fn admission_snapshot(&self) -> (SecondaryCustodyState, bool) {
        let state = super::super::lock_recover(&self.state);
        let ready = *state == SecondaryCustodyState::Ready
            && self.available.load(Ordering::Acquire)
            && super::super::lock_recover(&self.platform).is_some();
        (*state, ready)
    }

    #[cfg(test)]
    pub(super) fn is_wire_ready(&self) -> bool {
        self.wire_ready.load(Ordering::Acquire)
    }

    pub(super) fn bind_registry(&self, registry: Weak<SecondaryCustodyEntries>) {
        *super::super::lock_recover(&self.registry) = Some(registry);
    }

    pub(super) fn set_state(&self, state: SecondaryCustodyState) {
        let _ = self.transition_state(state);
    }

    pub(super) fn transition_fault(&self, state: SecondaryCustodyState) -> bool {
        self.transition_state(state)
    }

    fn transition_state(&self, state: SecondaryCustodyState) -> bool {
        let mut current = super::super::lock_recover(&self.state);
        if matches!(
            *current,
            SecondaryCustodyState::ReleaseAckPending
                | SecondaryCustodyState::Released
                | SecondaryCustodyState::ReleaseUncertain
                | SecondaryCustodyState::Superseded
        ) {
            return false;
        }
        if matches!(
            state,
            SecondaryCustodyState::ReleaseAckPending
                | SecondaryCustodyState::Released
                | SecondaryCustodyState::ReleaseUncertain
                | SecondaryCustodyState::Superseded
                | SecondaryCustodyState::Disconnected
                | SecondaryCustodyState::Failed
        ) {
            self.available.store(false, Ordering::Release);
        }
        *current = state;
        self.state_events.send_replace(state);
        true
    }

    pub(super) fn begin_settlement(&self) -> bool {
        !self.settlement_active.swap(true, Ordering::AcqRel)
    }

    pub(super) fn end_settlement(&self) {
        self.settlement_active.store(false, Ordering::Release);
    }

    pub(super) fn state(&self) -> SecondaryCustodyState {
        *super::super::lock_recover(&self.state)
    }

    pub(super) fn guardian_monitor_retired(&self) -> bool {
        matches!(
            self.state(),
            SecondaryCustodyState::ReleaseAckPending
                | SecondaryCustodyState::Released
                | SecondaryCustodyState::ReleaseUncertain
                | SecondaryCustodyState::Superseded
        )
    }

    pub(super) fn publish_state(&self, state: SecondaryCustodyState) {
        self.state_events.send_replace(state);
    }

    pub(super) fn subscribe_state(&self) -> tokio::sync::watch::Receiver<SecondaryCustodyState> {
        self.state_events.subscribe()
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../test_support/unit/secondary_custody/secondary_custody_lease_test_support.rs");
