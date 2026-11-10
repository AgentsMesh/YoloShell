use super::FaultReap;
use crate::worker::pty_force_termination::ForceTransactionState;
use crate::worker::pty_process_custody::PtyProcessCustody;
use std::sync::atomic::{AtomicU64, Ordering};

const FAULT_REAP_DIAGNOSTIC: &str = "adversarial fault reap diagnostic";
const FORCE_UNSTARTED: &str = "unstarted";
const FORCE_STARTED: &str = "started";
const CUSTODY_MISSING: &str = "missing";
const CUSTODY_TRACKING: &str = "tracking";
const CUSTODY_FREEZING: &str = "freezing";
const CUSTODY_FROZEN: &str = "frozen";
const CUSTODY_KILLING: &str = "killing";
const CUSTODY_SURVIVOR: &str = "survivor";
static IDLE_ACTIVITY: AtomicU64 = AtomicU64::new(0);
static SCAN_CONTINUATION_ACTIVITY: AtomicU64 = AtomicU64::new(0);
static SEMANTIC_PROGRESS_ACTIVITY: AtomicU64 = AtomicU64::new(0);

pub(super) fn record_activity(activity: super::schedule::FaultReapActivity) {
    let counter = match activity {
        super::schedule::FaultReapActivity::Idle => &IDLE_ACTIVITY,
        super::schedule::FaultReapActivity::ScanContinuation => &SCAN_CONTINUATION_ACTIVITY,
        super::schedule::FaultReapActivity::SemanticProgress => &SEMANTIC_PROGRESS_ACTIVITY,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

impl FaultReap {
    pub(super) fn publish_adversarial_diagnostic(&self) {
        let ownership = self.custody();
        let force = match ownership.force.transaction_state() {
            ForceTransactionState::Unstarted => FORCE_UNSTARTED,
            ForceTransactionState::Started => FORCE_STARTED,
        };
        let custody = ownership
            .processes
            .as_ref()
            .map(custody_snapshot)
            .unwrap_or_else(|| CUSTODY_MISSING.to_owned());
        self.state_observer.publish_adversarial_diagnostic(format!(
            "{FAULT_REAP_DIAGNOSTIC}: force={force} requested={} observer_only={} \
             wait_authority={} root_exit_proof={} root_gone_without_wait={} \
             activity_idle={} activity_scan={} activity_semantic={} custody={custody}",
            ownership.force.force_was_requested(),
            self.is_observer_only(),
            ownership.authority.is_some(),
            ownership.root_exit_proof.is_some(),
            self.root_gone_without_wait,
            IDLE_ACTIVITY.load(Ordering::Relaxed),
            SCAN_CONTINUATION_ACTIVITY.load(Ordering::Relaxed),
            SEMANTIC_PROGRESS_ACTIVITY.load(Ordering::Relaxed),
        ));
    }
}

fn custody_snapshot(custody: &PtyProcessCustody) -> String {
    match custody {
        PtyProcessCustody::Tracking(tree) => {
            format!(
                "{CUSTODY_TRACKING} {}",
                tree.adversarial_authority_snapshot()
            )
        }
        PtyProcessCustody::Freezing(tree) => {
            format!(
                "{CUSTODY_FREEZING} {}",
                tree.adversarial_authority_snapshot()
            )
        }
        PtyProcessCustody::Frozen(tree) => {
            format!("{CUSTODY_FROZEN} {}", tree.adversarial_authority_snapshot())
        }
        PtyProcessCustody::Killing(tree) => {
            format!(
                "{CUSTODY_KILLING} {}",
                tree.adversarial_authority_snapshot()
            )
        }
        PtyProcessCustody::Survivor(_) => CUSTODY_SURVIVOR.to_owned(),
        PtyProcessCustody::GonePending(_) => super::constants::CUSTODY_GONE_PENDING.to_owned(),
        PtyProcessCustody::GoneReported(_) => super::constants::CUSTODY_GONE_REPORTED.to_owned(),
        PtyProcessCustody::MasterClosePending(_) => {
            super::constants::CUSTODY_MASTER_CLOSE_PENDING.to_owned()
        }
    }
}
