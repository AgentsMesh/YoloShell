use super::super::constants::MAX_PTY_PROOF_FILE_DESCRIPTORS;
use super::macos_contract::{CapabilityProofError, CapabilityProofResult, CapabilityScanBudget};
use super::macos_fds::{list_descriptors, vnode_identity};
use super::macos_object_identity::MacosPtyObjectIdentity;
use anyhow::anyhow;

const DESCRIPTOR_SNAPSHOT_CHANGED_DETAIL: &str =
    "PTY proof descriptor snapshot changed while proving absence";

#[derive(Debug, Eq, PartialEq)]
struct DescriptorFingerprint {
    descriptor: i32,
    descriptor_type: u32,
    identity: Option<MacosPtyObjectIdentity>,
}

enum DescriptorPass {
    Matched,
    Absent(Vec<DescriptorFingerprint>),
}

pub(super) fn has_stable_tty_descriptor(
    pid: u32,
    tty_identity: MacosPtyObjectIdentity,
    budget: &mut CapabilityScanBudget,
) -> CapabilityProofResult<bool> {
    has_stable_tty_descriptor_with(
        tty_identity,
        budget,
        || list_descriptors(pid, MAX_PTY_PROOF_FILE_DESCRIPTORS),
        |descriptor| vnode_identity(pid, descriptor),
    )
}

pub(super) fn observes_tty_descriptor(
    pid: u32,
    tty_identity: MacosPtyObjectIdentity,
    budget: &mut CapabilityScanBudget,
) -> CapabilityProofResult<bool> {
    observes_tty_descriptor_with(
        tty_identity,
        budget,
        || list_descriptors(pid, MAX_PTY_PROOF_FILE_DESCRIPTORS),
        |descriptor| vnode_identity(pid, descriptor),
    )
}

fn observes_tty_descriptor_with<L, V>(
    tty_identity: MacosPtyObjectIdentity,
    budget: &mut CapabilityScanBudget,
    mut list: L,
    mut vnode: V,
) -> CapabilityProofResult<bool>
where
    L: FnMut() -> CapabilityProofResult<Vec<libc::proc_fdinfo>>,
    V: FnMut(i32) -> CapabilityProofResult<Option<MacosPtyObjectIdentity>>,
{
    let descriptors = list()?;
    budget.admit_descriptors(descriptors.len())?;
    let vnode_type = libc::PROX_FDTYPE_VNODE as u32;
    for descriptor in descriptors {
        if descriptor.proc_fdtype == vnode_type && vnode(descriptor.proc_fd)? == Some(tty_identity)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn has_stable_tty_descriptor_with<L, V>(
    tty_identity: MacosPtyObjectIdentity,
    budget: &mut CapabilityScanBudget,
    mut list: L,
    mut vnode: V,
) -> CapabilityProofResult<bool>
where
    L: FnMut() -> CapabilityProofResult<Vec<libc::proc_fdinfo>>,
    V: FnMut(i32) -> CapabilityProofResult<Option<MacosPtyObjectIdentity>>,
{
    let first = match descriptor_pass(tty_identity, budget, &mut list, &mut vnode)? {
        DescriptorPass::Matched => return Ok(true),
        DescriptorPass::Absent(fingerprint) => fingerprint,
    };
    let second = match descriptor_pass(tty_identity, budget, &mut list, &mut vnode)? {
        DescriptorPass::Matched => return Ok(true),
        DescriptorPass::Absent(fingerprint) => fingerprint,
    };
    if first != second {
        return Err(CapabilityProofError::candidate(anyhow!(
            DESCRIPTOR_SNAPSHOT_CHANGED_DETAIL
        )));
    }
    Ok(false)
}

fn descriptor_pass<L, V>(
    tty_identity: MacosPtyObjectIdentity,
    budget: &mut CapabilityScanBudget,
    list: &mut L,
    vnode: &mut V,
) -> CapabilityProofResult<DescriptorPass>
where
    L: FnMut() -> CapabilityProofResult<Vec<libc::proc_fdinfo>>,
    V: FnMut(i32) -> CapabilityProofResult<Option<MacosPtyObjectIdentity>>,
{
    let mut descriptors = list()?;
    budget.admit_descriptors(descriptors.len())?;
    descriptors.sort_unstable_by_key(|entry| (entry.proc_fd, entry.proc_fdtype));
    let vnode_type = libc::PROX_FDTYPE_VNODE as u32;
    let mut fingerprint = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        let identity = if descriptor.proc_fdtype == vnode_type {
            vnode(descriptor.proc_fd)?
        } else {
            None
        };
        if identity == Some(tty_identity) {
            return Ok(DescriptorPass::Matched);
        }
        fingerprint.push(DescriptorFingerprint {
            descriptor: descriptor.proc_fd,
            descriptor_type: descriptor.proc_fdtype,
            identity,
        });
    }
    Ok(DescriptorPass::Absent(fingerprint))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/descriptor/pty_session_descriptor_scan_macos_tests.rs"]
mod tests;
