pub(crate) const TEST_PRIVATE_DIRECTORY_MODE: u32 = 0o700;

#[path = "directory.rs"]
mod test_directory;
use test_directory::HostTestDirectory;

#[cfg(target_os = "linux")]
#[path = "linux_retirement_evidence.rs"]
pub(crate) mod linux_retirement_evidence;

pub(crate) fn reaped_host_test_process_identity() -> crate::process::ProcessIdentity {
    let mut command = std::process::Command::new(crate::test_executables::CAT);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null());
    let mut child = crate::process::spawn_waitable_test_child(command);
    let identity = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
    identity
}

pub(crate) fn test_custody_offer(
    request: &crate::pty_custody::CustodyRequest,
    descriptors: crate::pty_custody::CustodyDescriptorSet,
    digest: crate::sha256_digest::Sha256Digest,
) -> crate::pty_custody::CustodyOffer {
    let profile = match descriptors {
        crate::pty_custody::CustodyDescriptorSet::ReleasedSuccessor => {
            crate::pty_custody::CustodyProfile::ReleasedTerminal
        }
        _ => crate::pty_custody::CustodyProfile::ActiveRunning,
    };
    crate::pty_custody::CustodyOffer::new_with_profile(request, descriptors, profile, digest)
        .unwrap()
}

pub(crate) fn test_arm_custody(
    pending: &crate::pty_custody::CustodyDurableRecord,
    request: &crate::pty_custody::CustodyRequest,
    offer: &crate::pty_custody::CustodyOffer,
) -> Option<crate::pty_custody::CustodyDurableRecord> {
    if !matches!(
        offer.descriptor_set,
        crate::pty_custody::CustodyDescriptorSet::MacosProcessCapability { .. }
    ) {
        return pending.arm(request, offer);
    }
    let guardian = crate::pty_custody::CustodyGuardianBinding::new(
        uuid::Uuid::new_v4(),
        request.holder(),
        request,
        offer,
    )?;
    pending
        .prepare_guardian(request, offer, guardian)?
        .arm_prepared_guardian(request, offer, guardian)
}
