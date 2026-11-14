use super::{Admission, REQUEST_DETAIL};
use crate::pty_custody::{CustodyDurableRecord, CustodyOffer, CustodyRequest};
use anyhow::{Context, Result};

pub(crate) fn build(
    admission: Admission,
    durable: &CustodyDurableRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    prepared: Option<&CustodyDurableRecord>,
) -> Result<(CustodyDurableRecord, CustodyDurableRecord)> {
    if let Some(prepared) = prepared {
        let guardian = prepared
            .prepared_guardian_candidate()
            .and_then(|candidate| candidate.guardian)
            .context(REQUEST_DETAIL)?;
        let armed = prepared
            .arm_prepared_guardian(request, offer, guardian)
            .context(REQUEST_DETAIL)?;
        return Ok((prepared.clone(), armed));
    }
    let armed = match admission {
        Admission::Initial => durable.arm(request, offer),
        Admission::Successor => durable.rearm(request, offer),
    }
    .context(REQUEST_DETAIL)?;
    Ok((durable.clone(), armed))
}
