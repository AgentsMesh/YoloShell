use super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::Result;

const PREDECESSOR_DURABLE_MISMATCH_DETAIL: &str =
    "active PTY custody predecessor does not match its durable armed lease";

pub(crate) fn validate_durable(
    predecessor: Option<ReadyWorkerCustodyIdentity>,
    durable: &CustodyDurableRecord,
) -> Result<()> {
    let exact = predecessor.is_none_or(|identity| {
        durable
            .armed_lease()
            .or_else(|| durable.prepared_guardian_predecessor_lease())
            .is_some_and(|lease| identity.matches_durable(lease))
    });
    anyhow::ensure!(exact, PREDECESSOR_DURABLE_MISMATCH_DETAIL);
    Ok(())
}
