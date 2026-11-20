use super::CustodyHolderIdentity;
use yolo_process_identity::ProcessIdentityObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CustodyHolderObservation {
    Current,
    ProvenGone,
    Indeterminate,
}

pub struct ExactCustodyHolderGone {
    _private: (),
}

impl CustodyHolderIdentity {
    pub fn capture() -> anyhow::Result<Self> {
        let pid = std::process::id();
        Ok(Self {
            pid,
            start_time: yolo_process_identity::process_start_time(pid)?,
        })
    }

    pub fn observation(self) -> CustodyHolderObservation {
        match yolo_process_identity::observe_process_identity(self.pid, self.start_time) {
            ProcessIdentityObservation::Matching => CustodyHolderObservation::Current,
            ProcessIdentityObservation::Gone => CustodyHolderObservation::ProvenGone,
            ProcessIdentityObservation::Unknown => CustodyHolderObservation::Indeterminate,
        }
    }

    pub fn matches(self) -> bool {
        self.observation() == CustodyHolderObservation::Current
    }

    pub fn prove_gone(self) -> Option<ExactCustodyHolderGone> {
        (self.observation() == CustodyHolderObservation::ProvenGone)
            .then_some(ExactCustodyHolderGone { _private: () })
    }
}
