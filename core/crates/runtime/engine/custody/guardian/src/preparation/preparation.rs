use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{CustodyDurableRecord, CustodyGuardianPreparation};
#[cfg(test)]
use crate::pty_custody::{CustodyGuardianBinding, CustodyOffer, CustodyRequest};
use anyhow::{Context, Result};

#[path = "preparation_install.rs"]
mod installation;
#[path = "preparation_reconcile.rs"]
mod reconcile;
#[path = "preparation_replace.rs"]
mod replacement;
#[path = "preparation_supersede.rs"]
mod supersede;

const PREPARATION_DETAIL: &str = "detached guardian durable preparation changed unexpectedly";

pub(super) use installation::{install, install_successor_guarded};
#[cfg(test)]
use installation::{install_observed, install_successor_observed};
use replacement::{replace, replace_guarded, replace_observed};

pub(super) fn abandon(
    namespace: &NamespaceLease,
    paths: &Paths,
    prepared: &CustodyDurableRecord,
    preparation: CustodyGuardianPreparation,
) -> Result<(CustodyDurableRecord, FileIdentity)> {
    let binding = preparation.binding();
    let predecessor = prepared
        .abandon_prepared_guardian(binding)
        .context(PREPARATION_DETAIL)?;
    let installed_identity = replace(
        namespace,
        paths,
        prepared,
        preparation.custody_file_identity(),
        &predecessor,
    )?;
    Ok((predecessor, installed_identity))
}

pub(super) fn arm(
    namespace: &NamespaceLease,
    paths: &Paths,
    prepared: &CustodyDurableRecord,
    preparation: CustodyGuardianPreparation,
) -> Result<(CustodyDurableRecord, FileIdentity)> {
    let armed = prepared
        .arm_exact_prepared_guardian(preparation.binding())
        .context(PREPARATION_DETAIL)?;
    let installed_identity = replace(
        namespace,
        paths,
        prepared,
        preparation.custody_file_identity(),
        &armed,
    )?;
    Ok((armed, installed_identity))
}

#[cfg(test)]
fn exact_retry(
    record: &CustodyDurableRecord,
    identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
    retained: CustodyGuardianPreparation,
) -> Result<CustodyGuardianPreparation> {
    anyhow::ensure!(
        supersede::exact_retry(record, identity, request, offer, binding, Some(retained),),
        PREPARATION_DETAIL
    );
    Ok(retained)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/preparation/preparation_durability_tests.rs"]
mod durability_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/preparation/preparation_guard_tests.rs"]
mod guard_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/preparation/preparation_tests.rs"]
mod tests;
