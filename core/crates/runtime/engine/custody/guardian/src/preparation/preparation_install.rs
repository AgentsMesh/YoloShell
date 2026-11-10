use super::{PREPARATION_DETAIL, replace_observed, supersede};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianBinding, CustodyGuardianPreparation, CustodyOffer,
    CustodyRequest,
};
use anyhow::Result;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

pub(in crate::custody_guardian) fn install(
    namespace: &NamespaceLease,
    paths: &Paths,
    predecessor: &CustodyDurableRecord,
    predecessor_identity: FileIdentity,
    runtime_identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
) -> Result<CustodyGuardianPreparation> {
    install_observed(
        namespace,
        paths,
        predecessor,
        predecessor_identity,
        runtime_identity,
        request,
        offer,
        binding,
        |_| {},
    )
}

pub(super) fn install_observed<F>(
    namespace: &NamespaceLease,
    paths: &Paths,
    predecessor: &CustodyDurableRecord,
    predecessor_identity: FileIdentity,
    runtime_identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
    after_commit: F,
) -> Result<CustodyGuardianPreparation>
where
    F: FnMut(FileIdentity),
{
    let prepared = supersede::plan(
        predecessor,
        predecessor_identity,
        request,
        offer,
        binding,
        None,
    )?;
    let supersede::Plan::Replace(prepared) = prepared else {
        anyhow::bail!(PREPARATION_DETAIL)
    };
    let installed_identity = replace_observed(
        namespace,
        paths,
        predecessor,
        predecessor_identity,
        &prepared,
        after_commit,
    )?;
    Ok(CustodyGuardianPreparation::new(
        binding,
        runtime_identity,
        predecessor_identity,
        installed_identity,
    ))
}

pub(in crate::custody_guardian) fn install_successor_guarded(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    predecessor: &CustodyDurableRecord,
    predecessor_identity: FileIdentity,
    runtime_identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
    retained: Option<CustodyGuardianPreparation>,
) -> Result<CustodyGuardianPreparation> {
    anyhow::ensure!(
        guard.terminal_id() == predecessor.runtime.terminal_id,
        PREPARATION_DETAIL
    );
    install_successor_with(
        predecessor,
        predecessor_identity,
        runtime_identity,
        request,
        offer,
        binding,
        retained,
        |prepared| {
            super::replace_guarded(guard, paths, predecessor, predecessor_identity, prepared)
        },
    )
}

#[cfg(test)]
pub(super) fn install_successor_observed<F>(
    namespace: &NamespaceLease,
    paths: &Paths,
    predecessor: &CustodyDurableRecord,
    predecessor_identity: FileIdentity,
    runtime_identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
    retained: Option<CustodyGuardianPreparation>,
    after_commit: F,
) -> Result<CustodyGuardianPreparation>
where
    F: FnMut(FileIdentity),
{
    install_successor_with(
        predecessor,
        predecessor_identity,
        runtime_identity,
        request,
        offer,
        binding,
        retained,
        |prepared| {
            replace_observed(
                namespace,
                paths,
                predecessor,
                predecessor_identity,
                prepared,
                after_commit,
            )
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn install_successor_with<F>(
    predecessor: &CustodyDurableRecord,
    predecessor_identity: FileIdentity,
    runtime_identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
    retained: Option<CustodyGuardianPreparation>,
    replace: F,
) -> Result<CustodyGuardianPreparation>
where
    F: FnOnce(&CustodyDurableRecord) -> Result<FileIdentity>,
{
    match supersede::plan(
        predecessor,
        predecessor_identity,
        request,
        offer,
        binding,
        retained,
    )? {
        supersede::Plan::Exact(preparation) => {
            Ok(preparation.with_runtime_file_identity(runtime_identity))
        }
        supersede::Plan::Replace(prepared) => {
            let installed_identity = replace(&prepared)?;
            Ok(CustodyGuardianPreparation::new(
                binding,
                runtime_identity,
                predecessor_identity,
                installed_identity,
            ))
        }
    }
}
