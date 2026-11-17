use super::*;

const RELEASE_CANCELLED_DETAIL: &str =
    "PTY secondary-custody acknowledgement was cancelled after local release commit";
const RELEASE_UNCERTAIN_CONTEXT: &str =
    "Host cannot confirm Worker receipt after local PTY custody release";
const RELEASE_TRANSITION_DETAIL: &str =
    "PTY secondary-custody acknowledgement resolved from an impossible state";

pub(super) struct PendingReleaseAck {
    lease: Arc<SecondaryCustodyLease>,
    resolved: bool,
}

impl PendingReleaseAck {
    pub(super) fn new(lease: &Arc<SecondaryCustodyLease>) -> Self {
        Self {
            lease: Arc::clone(lease),
            resolved: false,
        }
    }

    pub(super) fn acknowledge(mut self) {
        self.finish(SecondaryCustodyState::Released);
    }

    pub(super) fn fail(mut self, detail: String) {
        self.finish(SecondaryCustodyState::ReleaseUncertain);
        super::containment::fail_stop(
            &self.lease.request_drain,
            format!("{RELEASE_UNCERTAIN_CONTEXT}: {detail}"),
        );
    }

    fn finish(&mut self, outcome: SecondaryCustodyState) {
        self.resolved = true;
        if let Err(error) = self.lease.resolve_pending_release(outcome) {
            let detail = super::containment::display_detail(&error);
            super::containment::fail_stop(
                &self.lease.request_drain,
                format!("{RELEASE_TRANSITION_DETAIL}: {detail}"),
            );
        }
    }
}

impl Drop for PendingReleaseAck {
    fn drop(&mut self) {
        if self.resolved {
            return;
        }
        self.finish(SecondaryCustodyState::ReleaseUncertain);
        super::containment::fail_stop(
            &self.lease.request_drain,
            format!("{RELEASE_UNCERTAIN_CONTEXT}: {RELEASE_CANCELLED_DETAIL}"),
        );
    }
}
