use super::{GuardianState, RECORD_DETAIL};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Result, ensure};

pub(super) fn progress(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    state: &mut GuardianState,
) -> Result<Progress> {
    let path = paths.pty_custody(state.runtime().terminal_id);
    let (record, identity): (CustodyDurableRecord, FileIdentity) =
        namespace.read_json_with_identity(&path)?;
    ensure!(
        record.is_valid() && record.runtime == *state.runtime(),
        RECORD_DETAIL
    );
    super::progress_record(namespace, paths, custody, state, record, identity)
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::custody_guardian) enum Progress {
    Pending,
    Retired,
}
