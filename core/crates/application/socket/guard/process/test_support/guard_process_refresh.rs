use std::path::PathBuf;

pub(super) struct RefreshEvidence {
    arm_request: PathBuf,
    arm_acknowledgement: PathBuf,
    holder_ready: PathBuf,
    handoff_request: PathBuf,
    coordinator_retire_request: PathBuf,
    handoff_acknowledgement: PathBuf,
    historical_witness_acknowledgement: PathBuf,
}

#[derive(Debug)]
pub(crate) struct RefreshMilestones {
    arm_requested: bool,
    arm_acknowledged: bool,
    holder_ready: bool,
    handoff_requested: bool,
    coordinator_retire_requested: bool,
    handoff_acknowledged: bool,
    historical_witness_captured: bool,
}

impl RefreshEvidence {
    pub(super) fn new(
        arm_request: PathBuf,
        arm_acknowledgement: PathBuf,
        holder_ready: PathBuf,
        handoff_request: PathBuf,
        coordinator_retire_request: PathBuf,
        handoff_acknowledgement: PathBuf,
        historical_witness_acknowledgement: PathBuf,
    ) -> Self {
        Self {
            arm_request,
            arm_acknowledgement,
            holder_ready,
            handoff_request,
            coordinator_retire_request,
            handoff_acknowledgement,
            historical_witness_acknowledgement,
        }
    }

    pub(super) fn observe(&self) -> RefreshMilestones {
        RefreshMilestones {
            arm_requested: self.arm_request.exists(),
            arm_acknowledged: self.arm_acknowledgement.exists(),
            holder_ready: self.holder_ready.exists(),
            handoff_requested: self.handoff_request.exists(),
            coordinator_retire_requested: self.coordinator_retire_request.exists(),
            handoff_acknowledged: self.handoff_acknowledgement.exists(),
            historical_witness_captured: self.historical_witness_acknowledgement.exists(),
        }
    }
}

impl RefreshMilestones {
    pub(crate) fn complete(&self) -> bool {
        self.arm_requested
            && self.arm_acknowledged
            && self.holder_ready
            && self.handoff_requested
            && self.coordinator_retire_requested
            && self.handoff_acknowledged
            && self.historical_witness_captured
    }
}
