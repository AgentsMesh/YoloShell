use crate::process::{MacosProcessCapabilityObservation, MacosSecondaryCustody};
use anyhow::Result;

pub(super) enum ExactSettlement {
    Pending,
    Released,
    PrematurelyReleased,
}

pub(super) fn exact_settlement(custody: &MacosSecondaryCustody) -> Result<ExactSettlement> {
    match custody.closure_settled() {
        Ok(true) => Ok(ExactSettlement::Released),
        Ok(false) => Ok(if custody.request_active_settlement()? {
            ExactSettlement::Released
        } else {
            ExactSettlement::Pending
        }),
        Err(_error)
            if custody.process_capability_observation()
                == Some(MacosProcessCapabilityObservation::PrematurelyReleased) =>
        {
            Ok(ExactSettlement::PrematurelyReleased)
        }
        Err(error) => Err(error),
    }
}
