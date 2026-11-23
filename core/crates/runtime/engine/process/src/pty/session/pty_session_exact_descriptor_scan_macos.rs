use super::super::constants::MAX_PTY_PROOF_FILE_DESCRIPTORS;
use super::macos_contract::{CapabilityProofError, CapabilityProofResult, CapabilityScanBudget};
use super::macos_fds::{list_descriptors, vnode_identity};
use super::macos_object_identity::MacosPtyObjectIdentity;
use anyhow::anyhow;

const EXACT_DESCRIPTOR_CANDIDATE_COUNT: usize = 1;
const EXACT_DESCRIPTOR_CHANGED_DETAIL: &str =
    "exact PTY custody descriptor changed while proving its state";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExactDescriptorObservation {
    descriptor_type: Option<u32>,
    identity: Option<MacosPtyObjectIdentity>,
}

pub(in crate::process) fn capture_exact_descriptor_identity(
    pid: u32,
    descriptor: i32,
) -> anyhow::Result<Option<MacosPtyObjectIdentity>> {
    let mut budget = CapabilityScanBudget::start(EXACT_DESCRIPTOR_CANDIDATE_COUNT)
        .map_err(anyhow::Error::new)?;
    Ok(observe_exact_descriptor(
        descriptor,
        &mut budget,
        || list_descriptors(pid, MAX_PTY_PROOF_FILE_DESCRIPTORS),
        |candidate| vnode_identity(pid, candidate),
    )
    .map_err(anyhow::Error::new)?
    .matching_vnode())
}

pub(in crate::process) fn has_stable_exact_descriptor(
    pid: u32,
    descriptor: i32,
    identity: MacosPtyObjectIdentity,
) -> anyhow::Result<bool> {
    let mut budget = CapabilityScanBudget::start(EXACT_DESCRIPTOR_CANDIDATE_COUNT)
        .map_err(anyhow::Error::new)?;
    has_stable_exact_descriptor_with(
        descriptor,
        identity,
        &mut budget,
        || list_descriptors(pid, MAX_PTY_PROOF_FILE_DESCRIPTORS),
        |candidate| vnode_identity(pid, candidate),
    )
    .map_err(anyhow::Error::new)
}

fn has_stable_exact_descriptor_with<L, V>(
    descriptor: i32,
    identity: MacosPtyObjectIdentity,
    budget: &mut CapabilityScanBudget,
    mut list: L,
    mut vnode: V,
) -> CapabilityProofResult<bool>
where
    L: FnMut() -> CapabilityProofResult<Vec<libc::proc_fdinfo>>,
    V: FnMut(i32) -> CapabilityProofResult<Option<MacosPtyObjectIdentity>>,
{
    let first = observe_exact_descriptor(descriptor, budget, &mut list, &mut vnode)?;
    let second = observe_exact_descriptor(descriptor, budget, &mut list, &mut vnode)?;
    if first != second {
        return Err(CapabilityProofError::candidate(anyhow!(
            EXACT_DESCRIPTOR_CHANGED_DETAIL
        )));
    }
    Ok(first.matching_vnode() == Some(identity))
}

fn observe_exact_descriptor<L, V>(
    descriptor: i32,
    budget: &mut CapabilityScanBudget,
    mut list: L,
    mut vnode: V,
) -> CapabilityProofResult<ExactDescriptorObservation>
where
    L: FnMut() -> CapabilityProofResult<Vec<libc::proc_fdinfo>>,
    V: FnMut(i32) -> CapabilityProofResult<Option<MacosPtyObjectIdentity>>,
{
    let descriptors = list()?;
    budget.admit_descriptors(descriptors.len())?;
    let Some(entry) = descriptors
        .into_iter()
        .find(|entry| entry.proc_fd == descriptor)
    else {
        return Ok(ExactDescriptorObservation::absent());
    };
    let vnode_type = libc::PROX_FDTYPE_VNODE as u32;
    let identity = (entry.proc_fdtype == vnode_type)
        .then(|| vnode(entry.proc_fd))
        .transpose()?
        .flatten();
    Ok(ExactDescriptorObservation {
        descriptor_type: Some(entry.proc_fdtype),
        identity,
    })
}

impl ExactDescriptorObservation {
    fn absent() -> Self {
        Self {
            descriptor_type: None,
            identity: None,
        }
    }

    fn matching_vnode(self) -> Option<MacosPtyObjectIdentity> {
        (self.descriptor_type == Some(libc::PROX_FDTYPE_VNODE as u32))
            .then_some(self.identity)
            .flatten()
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/pty_session_exact_descriptor_scan_macos_tests.rs"]
mod tests;
