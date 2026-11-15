use super::Admission;
use crate::model::RuntimeRecord;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableState, CustodyHolderObservation, CustodyRequest,
};

pub(crate) fn matches(
    admission: Admission,
    request: &CustodyRequest,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
) -> bool {
    match admission {
        Admission::Initial => request.controller_generation == runtime.controller_generation,
        Admission::Successor => match &durable.state {
            CustodyDurableState::Armed {
                holder,
                controller_generation,
                ..
            }
            | CustodyDurableState::Retired {
                holder,
                controller_generation,
                ..
            } => {
                holder.observation() == CustodyHolderObservation::ProvenGone
                    && request.controller_generation > *controller_generation
                    && request.controller_generation > runtime.controller_generation
            }
            CustodyDurableState::PreparedGuardian { candidate, .. } => {
                let guardian_known = candidate.guardian.is_some_and(|guardian| {
                    guardian.process().observation() != CustodyHolderObservation::Indeterminate
                });
                durable.prepared_guardian_predecessor_lease().is_some()
                    && candidate.holder.observation() == CustodyHolderObservation::ProvenGone
                    && guardian_known
                    && request.controller_generation > candidate.controller_generation
                    && request.controller_generation > runtime.controller_generation
            }
            CustodyDurableState::Pending {} => false,
        },
    }
}
